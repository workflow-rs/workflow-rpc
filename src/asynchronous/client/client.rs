use borsh::{BorshSerialize,BorshDeserialize};
use ahash::AHashMap;
use std::{
    mem::size_of, 
    sync::{Arc, Mutex, atomic::{AtomicBool, AtomicU64, Ordering}}, 
    time::{Instant, Duration},
    marker::Send
};
use futures::{
    future::FutureExt, // for `.fuse()`
    pin_mut,
    select,
};
use workflow_websocket::client::{
    WebSocket,
    Settings as WebSocketSettings,
    Message as WebSocketMessage,
    Error as WebSocketError,
};
// use crate::asynchronous::client::*;
use super::*;
use super::error::Error;
use super::result::Result;
// use crate::asynchronous::client::error::Error;
// use crate::asynchronous::client::result::Result;
// use crate::message::*;
// use crate::error::*;
use workflow_log::{log_error, log_trace};
use workflow_core::channel::*;
use workflow_core::trigger::*;

pub use workflow_websocket::client::Ctl;

const STATUS_SUCCESS: u32 = 0;
const STATUS_ERROR: u32 = 1;

const RPC_CTL_RECEIVER_SHUTDOWN: u32 = 0;

// pub type RpcResponseFn = Arc<Box<(dyn Fn(Result<Option<&[u8]>>) + Sync + Send)>>;
pub type RpcResponseFn = Arc<Box<(dyn Fn(Result<&[u8]>) + Sync + Send)>>;




struct Pending {
    timestamp : Instant,
    callback : RpcResponseFn,
}

impl Pending {
    fn new(callback: RpcResponseFn) -> Self {
        Self {
            timestamp: Instant::now(),
            callback,
        }
    }
}

pub struct Inner {
    ws : WebSocket,
    is_open : AtomicBool,
    pending : Arc<Mutex<AHashMap<u64, Pending>>>,
    receiver_is_running : AtomicBool,
    timeout_is_running : AtomicBool,
    receiver_shutdown : SingleTrigger,
    timeout_shutdown : ReqRespTrigger,
    timeout_timer_interval : AtomicU64,
    timeout_duration : AtomicU64,
    ctl_channel : Mutex<Option<(Sender<Ctl>, Receiver<Ctl>)>>,
}

impl Inner {
    fn new(url : &str) -> Result<Self> {
        let inner = Inner {
            ws: WebSocket::new(url, WebSocketSettings::default())?,
            pending: Arc::new(Mutex::new(AHashMap::new())),
            is_open : AtomicBool::new(false),
            receiver_is_running : AtomicBool::new(false),
            receiver_shutdown : SingleTrigger::new(),
            timeout_is_running : AtomicBool::new(false),
            timeout_shutdown : ReqRespTrigger::new(),
            timeout_duration : AtomicU64::new(60_000),
            timeout_timer_interval : AtomicU64::new(5_000),
            ctl_channel : Mutex::new(None),
        };

        Ok(inner)
    }

    fn timeout_task(self : Arc<Self>) {   
        self.timeout_is_running.store(true, Ordering::SeqCst);
        workflow_core::task::spawn(async move {
            
            let shutdown = self.timeout_shutdown.request.listener.clone().fuse();
            pin_mut!(shutdown);

            loop {
                
                let timeout_timer_interval = Duration::from_millis(self.timeout_timer_interval.load(Ordering::SeqCst));
                let delay = async_std::task::sleep(timeout_timer_interval).fuse();
                pin_mut!(delay);

                select! {
                    () = shutdown => { break; },
                    () = delay => {
                        let mut pending = self.pending.lock().unwrap();
                        let mut purge = Vec::<u64>::new();
                        let timeout = Duration::from_millis(self.timeout_duration.load(Ordering::Relaxed));
                        for (id,pending) in pending.iter() {
                            if pending.timestamp.elapsed() > timeout {
                                purge.push(*id);
                                (pending.callback)(Err(Error::Timeout));
                            }
                        }
                        for id in purge.iter() {
                            pending.remove(id);
                        }
                    },
                }
            }

            self.timeout_is_running.store(false,Ordering::SeqCst);
            self.timeout_shutdown.response.trigger.trigger();
        });

    }

    fn receiver_task(self : Arc<Self>) {
        self.receiver_is_running.store(true,Ordering::SeqCst);
        let receiver_rx = self.ws.receiver_rx().clone();
        workflow_core::task::spawn(async move {

            loop {
                let message = receiver_rx.recv().await.unwrap();

                match message {
                    WebSocketMessage::Binary(data) => {
                        self.handle_binary_response(&data);
                    },
                    WebSocketMessage::Text(_text) => {
                        // self.handle_json_response(text);
                    },
                    WebSocketMessage::Ctl(ctl) => {
                        match ctl {
                            Ctl::Open => {
                                self.is_open.store(true,Ordering::SeqCst);
                            },
                            Ctl::Closed => {
                                self.is_open.store(false,Ordering::SeqCst);
                            },
                            Ctl::RpcCtl(RPC_CTL_RECEIVER_SHUTDOWN) => {
                                break;
                            },
                            _ => { }
                        }

                        let sender = match self.ctl_channel.lock().unwrap().as_ref() {
                            Some(channel) => Some(channel.0.clone()),
                            None => None
                        };

                        if let Some(sender) = sender {
                            sender.clone().send(ctl).await.unwrap();
                        }
                    }
                }
            }

            self.receiver_is_running.store(false,Ordering::SeqCst);
            self.receiver_shutdown.trigger.trigger();//send(()).await;
        });
    }


    fn handle_binary_response(&self, response : &[u8]) {

        if response.len() < size_of::<RespHeader>() {
            log_error!("RPC receiving response with {} bytes, which is smaller than required header size of {} bytes", response.len(), size_of::<ReqHeader>());
        }

        let msg = RespMessage::try_from(response);
        match msg {
            Ok(msg) => {

                match self.pending.lock().unwrap().remove(&msg.id) {
                    Some(pending) => {

                        match msg.status {
                            STATUS_SUCCESS  => { 
                                (pending.callback)(Ok(msg.data)); 
                            },
                            STATUS_ERROR => {
                                if let Ok(err) = RpcResponseError::try_from_slice(msg.data) {
                                    (pending.callback)(Err(Error::RpcCall(err)));
                                } else {
                                    (pending.callback)(Err(Error::ErrorDeserializingResponseData));
                                }
                            }
                            code  => { 
                                (pending.callback)(Err(Error::StatusCode(code))) 
                            },
                        }
                    },
                    None => {
                        log_trace!("rpc callback with id {} not found", msg.id);
                    }
                }
        
            },
            Err(err) => {
                log_error!("Failed to decode rpc server response: {:?}", err);
            }
        }
    }   
    
    async fn stop_receiver(&self) -> Result<()> {
        if self.receiver_is_running.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.ws.inject_ctl(Ctl::RpcCtl(RPC_CTL_RECEIVER_SHUTDOWN)).map_err(|_| { Error::ReceiverCtl })?;
        self.receiver_shutdown.listener.clone().await;

        Ok(())
    }

    async fn stop_timeout(&self) -> Result<()> {
        if self.timeout_is_running.load(Ordering::SeqCst) != true {
            return Ok(());
        }

        self.timeout_shutdown.request.trigger.trigger();
        self.timeout_shutdown.response.listener.clone().await;
        
        Ok(())
    }

}

#[derive(Clone)]
pub struct RpcClient<Ops>
where
    // Arc<Inner> : Send + Sync,
    Ops : TryInto<u32> + Send + Sync + 'static,
{
    inner: Arc<Inner>,
    _ops_ : std::marker::PhantomData<Ops>,
}

impl<Ops> RpcClient<Ops>
where
    Ops : Into<u32> + Send + Sync + 'static
{
    pub fn new(url : &str) -> Result<RpcClient<Ops>> {

        let client = RpcClient{
            inner : Arc::new(Inner::new(url)?),
            _ops_ : std::marker::PhantomData,
        };

        client.inner.clone().timeout_task();
        client.inner.clone().receiver_task();

        Ok(client)
    }

    pub fn init_ctl(&self) -> Receiver<Ctl> {
        let channel = unbounded();
        let receiver = channel.1.clone();
        *self.inner.ctl_channel.lock().unwrap() = Some(channel);
        receiver
    }

    pub async fn connect(&self, block_until_connected:bool) -> Result<Option<Listener>> {
        Ok(self.inner.ws.connect(block_until_connected).await?)
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.inner.stop_timeout().await?;
        self.inner.stop_receiver().await?;
        Ok(())
    }

    pub fn is_open(&self) -> bool {
        self.inner.ws.is_open()
    }

    pub async fn call_callback_with_buffer(
        &self,
        op : Ops,
        message : Message<'_>,
        callback : RpcResponseFn
    ) -> Result<()> {
        if !self.is_open() {
            return Err(WebSocketError::NotConnected.into());
        }

        let mut pending = self.inner.pending.lock().unwrap();
        let id = u64::from_le_bytes(rand::random::<[u8; 8]>());
        pending.insert(id,Pending::new(callback));
        drop(pending);
        self.inner.ws.post(to_ws_msg((ReqHeader{op : op.into(),id},message))).await?;
        Ok(())
    }

    pub async fn call_async_with_buffer(
        &self,
        op : Ops,
        message : Message<'_>,
    ) -> Result<Vec<u8>> {
        if !self.is_open() {
            return Err(WebSocketError::NotConnected.into());
        }

        let id = u64::from_le_bytes(rand::random::<[u8; 8]>());
        let (sender,receiver) = oneshot();

        {
            let mut pending = self.inner.pending.lock().unwrap();
            pending.insert(id,Pending::new(Arc::new(Box::new(move |result| {
                let resp = match result {
                    Ok(data) => Ok(data.to_vec()),
                    Err(e) => Err(e),
                };
                sender.try_send(resp).unwrap();
            }))));
            drop(pending);
        }

        self.inner.ws.post(to_ws_msg((ReqHeader{op : op.into(),id},message))).await?;
        receiver.recv().await?
    }

    pub async fn call<Req,Resp>(
        &self,
        op : Ops,
        req : Req,
    ) -> Result<Resp>
    where
        Req : BorshSerialize + Send + Sync + 'static,
        Resp : BorshDeserialize + Send + Sync +'static,
    {
        let data = req.try_to_vec().map_err(|_| { Error::BorshSerialize })?;
        let resp = self.call_async_with_buffer(op, Message::Request(&data)).await?;
        Ok(Resp::try_from_slice(&resp).map_err(|e|Error::SerdeDeserialize(e.to_string()))?)
    }

}


