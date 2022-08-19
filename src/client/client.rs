use borsh::BorshDeserialize;
use ahash::AHashMap;
use std::{mem::size_of, sync::{Arc, Mutex}, time::{Instant, Duration}};
use futures::{
    future::FutureExt, // for `.fuse()`
    pin_mut,
    select,
};
use workflow_websocket::client::{
    WebSocket, Ctl,
    Settings as WebSocketSettings,
    Message as WebSocketMessage,
};
use crate::client::error::Error;
use crate::message::*;
use crate::error::*;
use workflow_log::{log_error, log_trace};
use workflow_core::sync::reqresp::ReqResp;
use workflow_core::sync::oneshot::Oneshot;

const STATUS_SUCCESS: u32 = 0;
const STATUS_ERROR: u32 = 1;

pub type RpcResponseFn = Arc<Box<(dyn Fn(Result<Option<&[u8]>,Error>) + Sync + Send)>>;

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

#[derive(Clone)]
pub struct RpcClient {
    ws : WebSocket,
    is_open : Arc<Mutex<bool>>,
    pending : Arc<Mutex<AHashMap<u64, Pending>>>,
    receiver_is_running : Arc<Mutex<bool>>,
    timeout_is_running : Arc<Mutex<bool>>,
    receiver_shutdown : Oneshot,
    timeout_shutdown : ReqResp,
    timeout_timer_interval : Duration,
    timeout_duration : Duration,
}

impl RpcClient {
    pub fn new(url : &str) -> Result<Arc<RpcClient>,Error> {

        let client = Arc::new(RpcClient{
            ws: WebSocket::new(url, WebSocketSettings::default())?,
            pending: Arc::new(Mutex::new(AHashMap::new())),
            is_open : Arc::new(Mutex::new(false)),
            receiver_is_running : Arc::new(Mutex::new(false)),
            receiver_shutdown : Oneshot::new(),
            timeout_is_running : Arc::new(Mutex::new(false)),
            timeout_shutdown : ReqResp::new(),
            timeout_duration : Duration::from_millis(60_000),
            timeout_timer_interval : Duration::from_millis(5_000),
        });

        client.timeout_task();
        client.receiver_task();

        Ok(client)
    }

    pub async fn shutdown(self : &Arc<Self>) -> Result<(),Error> {
        self.stop_timeout().await?;
        self.stop_receiver().await?;
        Ok(())
    }

    pub async fn dispatch(
        self : &Arc<Self>,
        op : u32,
        message : Message<'_>,
        callback : RpcResponseFn
    ) -> Result<(),Error> {

        let mut pending = self.pending.lock().unwrap();
        let id = u64::from_le_bytes(rand::random::<[u8; 8]>());
        pending.insert(id,Pending::new(callback));
        drop(pending);
        self.ws.send(to_ws_msg((ReqHeader{op,id},message))).await?;
        Ok(())
    }

    fn timeout_task(self : &Arc<Self>) {
        
        *self.timeout_is_running.lock().unwrap() = true;
        let self_ = self.clone();
        workflow_core::sync::task::spawn(async move {
            
            let shutdown = self_.timeout_shutdown.request.take_future().fuse();
            pin_mut!(shutdown);

            loop {
                
                let delay = async_std::task::sleep(self_.timeout_timer_interval).fuse();
                pin_mut!(delay);

                select! {
                    () = shutdown => { break; },
                    () = delay => {
                        let mut pending = self_.pending.lock().unwrap();
                        let mut purge = Vec::<u64>::new();
                        for (id,pending) in pending.iter() {
                            if pending.timestamp.elapsed() > self_.timeout_duration {
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

            *self_.timeout_is_running.lock().unwrap() = false;
            self_.timeout_shutdown.response.send(()).await;
        });

    }

    fn receiver_task(self : &Arc<Self>) {
        *self.receiver_is_running.lock().unwrap() = true;
        let receiver_rx = self.ws.receiver_rx.clone();
        let self_ = self.clone();
        workflow_core::sync::task::spawn(async move {

            loop {
                let message = receiver_rx.recv().await.unwrap();

                match message {
                    WebSocketMessage::Binary(data) => {
                        self_.handle_response(&data);
                    },
                    WebSocketMessage::Ctl(ctl) => {
                        match ctl {
                            Ctl::Open => {
                                *self_.is_open.lock().unwrap() = true;
                            },
                            Ctl::Closed => {
                                *self_.is_open.lock().unwrap() = false;
                            },
                            Ctl::Shutdown => {
                                break;
                            },
                            _ => { }
                        }
                    },
                    _ => {

                    }
                }
            }

            *self_.receiver_is_running.lock().unwrap() = false;
            self_.receiver_shutdown.send(()).await;
        });
    }

    async fn stop_receiver(self : &Arc<Self>) -> Result<(),Error> {
        if *self.receiver_is_running.lock().unwrap() != true {
            return Ok(());
        }

        self.ws.receiver_tx.send(WebSocketMessage::Ctl(Ctl::Shutdown)).await.map_err(|_| { Error::ReceiverCtl })?;
        self.receiver_shutdown.recv().await;

        Ok(())
    }

    async fn stop_timeout(self : &Arc<Self>) -> Result<(),Error> {
        if *self.timeout_is_running.lock().unwrap() != true {
            return Ok(());
        }

        self.timeout_shutdown.request.send(()).await;
        self.timeout_shutdown.response.recv().await;
        
        Ok(())
    }

    fn handle_response(self : &Arc<Self>, response : &[u8]) {

        if response.len() < size_of::<RespHeader>() {
            log_error!("RPC receiving response with {} bytes, which is smaller than required header size of {} bytes", response.len(), size_of::<ReqHeader>());
        }

        let msg = RespMessage::try_from(response);
        match msg {
            Ok(msg) => {

                match self.pending.lock().unwrap().remove(&msg.id) {
                    Some(pending) => {

                        match msg.status {
                            STATUS_SUCCESS  => { (pending.callback)(Ok(msg.data)); },
                            STATUS_ERROR => {

                                match msg.data {
                                    Some(data) => {
                                        if let Ok(err) = RpcResponseError::try_from_slice(data) {
                                            (pending.callback)(Err(Error::RpcCall(err)));
                                        } else {
                                            (pending.callback)(Err(Error::ErrorDeserializingResponseData));
                                        }
                                    },
                                    None => {
                                        (pending.callback)(Err(Error::NoDataInErrorResponse));
                                    }
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

        let header: &RespHeader = unsafe { std::mem::transmute(&response[0]) };
        let id = header.id;
        let status = header.status;

        log_trace!("RECEIVING MESSAGE ID: {}  STATUS: {}", id, status);
    }
}


