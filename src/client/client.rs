use borsh::{BorshDeserialize, BorshSerialize};
use ahash::AHashMap;
use std::{mem::size_of, sync::{Arc, Mutex}};
use manual_future::{ManualFuture, ManualFutureCompleter};
use workflow_websocket::client::{
    WebSocket, Ctl,
    Settings as WebSocketSettings,
    Message as WebSocketMessage,
};
use crate::client::identifier::Identifier;
use crate::client::error::Error;
use crate::message::*;
use crate::error::*;
use workflow_log::{log_error, log_trace};

const STATUS_SUCCESS: u32 = 0;
const STATUS_ERROR: u32 = 1;

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub enum TestReq {
    First(u32),
    Second(u64),
    Third(String)
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub enum TestResp {
    First(u32),
    Second(u64),
    Third(String)
}

pub type RpcResponseFn = Arc<Box<(dyn Fn(Result<Option<&[u8]>,Error>) + Sync + Send)>>;

#[derive(Clone)]
pub struct RpcClient {
    id : Identifier,
    ws : WebSocket,
    is_open : Arc<Mutex<bool>>,
    receiver_is_running : Arc<Mutex<bool>>,
    receiver : Arc<Mutex<Option<ManualFuture<()>>>>,

    pending : Arc<Mutex<AHashMap<u64, RpcResponseFn>>>,
}

impl RpcClient {
    pub fn new(url : &str) -> Result<Arc<RpcClient>,Error> {

        let (receiver, completer) = ManualFuture::<()>::new();

        let client = Arc::new(RpcClient{
            id: Identifier::default(),
            ws: WebSocket::new(url, WebSocketSettings::default())?,
            pending: Arc::new(Mutex::new(AHashMap::new())),
            is_open : Arc::new(Mutex::new(false)),
            receiver_is_running : Arc::new(Mutex::new(false)),
            receiver : Arc::new(Mutex::new(Some(receiver))),
        });

        client.receiver_task(completer);

        Ok(client)
    }

    pub async fn shutdown(self : &Arc<Self>) -> Result<(),Error> {
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
        let id = self.id.next();
        pending.insert(id,callback);
        drop(pending);
        self.ws.send(to_ws_msg((ReqHeader{op,id},message))).await?;
        Ok(())
    }

    fn receiver_task(
        self : &Arc<Self>,
        completer : ManualFutureCompleter<()>,
    ) {
        *self.receiver_is_running.lock().unwrap() = true;
        let receiver_rx = self.ws.receiver_rx.clone();
        let self_ = self.clone();
        workflow_core::task::spawn(async move {

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
            completer.complete(()).await;
        });
    }

    async fn stop_receiver(self : &Arc<Self>) -> Result<(),Error> {
        if *self.receiver_is_running.lock().unwrap() != true {
            return Ok(());
        }

        self.ws.receiver_tx.send(WebSocketMessage::Ctl(Ctl::Shutdown)).await.map_err(|_| { Error::ReceiverCtl })?;
        let mut receiver = self.receiver.lock().unwrap();
        let receiver = receiver.as_mut().take().unwrap();
        receiver.await;

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
                    Some(callback) => {

                        match msg.status {
                            STATUS_SUCCESS  => { callback(Ok(msg.data)); },
                            STATUS_ERROR => {

                                match msg.data {
                                    Some(data) => {
                                        if let Ok(err) = RpcResponseError::try_from_slice(data) {
                                            callback(Err(Error::RpcCall(err)));
                                        } else {
                                            callback(Err(Error::ErrorDeserializingResponseData));
                                        }
                                    },
                                    None => {
                                        callback(Err(Error::NoDataInErrorResponse));
                                    }
                                }
                            }
                            code  => { 
                                callback(Err(Error::StatusCode(code))) 
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


