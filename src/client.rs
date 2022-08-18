use std::{fmt::Debug, marker::PhantomData};
use borsh::{BorshDeserialize, BorshSerialize};
use wasm_bindgen::prelude::*;
use ahash::AHashMap;
use std::{mem::size_of, sync::{Arc, Mutex}};
use manual_future::{ManualFuture, ManualFutureCompleter};
use thiserror::Error;
// TODO split core structs/enums into crate root for websocket
use workflow_websocket::client::{
    WebSocket,
    Error as WebSocketError,
    Settings as WebSocketSettings,
    Message as WebSocketMessage,
    Ctl
};

use workflow_core::enums::u32_try_from;
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

u32_try_from!{
    #[derive(Clone, Debug)]
    #[repr(u32)]
    pub enum RpcOps {
        Raw = 0,
        Borsh,
    }
}

#[derive(Error, Debug)]
pub enum Error {

    #[error("WebSocket error: {0}")]
    WebSocketError(#[from] WebSocketError),

    #[error("RPC request timeout")]
    Timeout,
    #[error("Receiver ctl failure")]
    ReceiverCtl,
    #[error("Invalid header size")]
    HeaderSize,
    // ~
    #[error("RPC: no data in success response")]
    NoDataInSuccessResponse,
    #[error("RPC: no data in error response")]
    NoDataInErrorResponse,
    #[error("RPC: error deserializing response data")]
    ErrorDeserializingResponseData,
    // Data(Vec<u8>),
    #[error("RPC: status code {0}")]
    StatusCode(u32),
    // RpcCall(u32,Option<Vec<u8>>),
    #[error("RPC: response error {0:?}")]
    RpcCall(RpcResponseError),
    // RpcCall(u32,Option<&[u8]>),
    #[error("RPC: borsh serialization error")]
    BorshSerialize,
    #[error("RPC: borsh deserialization error")]
    BorshDeserialize,
    
    // borsh
    #[error("RPC: borsh error deserializing response")]
    BorshResponseDeserialize,
}

impl Into<JsValue> for Error {
    fn into(self) -> JsValue {
        JsValue::from(format!("{}",self).to_string())
    }
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub enum RpcResponseError {
    ReqDeserialize,
    RespSerialize,
    Data(Vec<u8>),
}

#[derive(Clone, Copy)]
#[repr(packed)]
pub struct ReqHeader {
    id : u64,
    op : u32,
}

#[derive(Clone, Copy)]
#[repr(packed)]
pub struct RespHeader {
    id : u64,
    status : u32
}

u32_try_from! {
    pub enum RespStatus {
        Success = 0,
        Error = 1,
    }
}

pub enum Message<'data> {
    Request(&'data [u8]),
    Post(&'data [u8]),
}

impl<'data> Message<'data> {
    fn data(&self) -> &'data [u8] {
        match self {
            Message::Request(data) => data,
            Message::Post(data) => data,
        }
    }
}

fn to_ws_msg(msg : (ReqHeader, Message<'_>)) -> WebSocketMessage {
    let (header, message) = msg;
    let data = message.data();
    let len = data.len() + size_of::<ReqHeader>();
    let mut buffer = Vec::with_capacity(len);
    unsafe { buffer.set_len(len); }
    let dest_header: &mut ReqHeader = unsafe { std::mem::transmute(&mut buffer[0]) };
    *dest_header = header;
    buffer[size_of::<ReqHeader>()..].copy_from_slice(data);
    buffer.into()
}

pub type RpcResponseFn = Arc<Box<(dyn Fn(Result<Option<&[u8]>,Error>) + Sync + Send)>>;

#[derive(Debug, Clone)]
struct Identifier {
    v : Arc<Mutex<u64>>
}

impl Default for Identifier {
    fn default() -> Self {
        Self { v: Arc::new(Mutex::new(0)) }
    }
}

impl Into<u64> for Identifier {
    fn into(self) -> u64 {
        *self.v.lock().unwrap()
    }
}

impl Identifier {
    pub fn next(&self) -> u64 {
        let mut lock = self.v.lock().unwrap();
        let v = *lock;
        *lock += 1;
        v
    }
}

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


#[derive(Debug)]
pub enum RespError<T>
where
    T : BorshDeserialize
{
    NoData,
    Data(T),
    Rpc(Error),
}

#[derive(Clone)]
pub struct RpcClientBorsh<Req,Resp> 
where
    Req : Debug + Send + Sync + BorshSerialize + 'static,
    Resp : Debug + Send + Sync + BorshDeserialize + 'static,
{
    rpc : Arc<RpcClient>,
    _req_ : PhantomData<Req>,
    _resp_ : PhantomData<Resp>,
}

impl<Req,Resp> RpcClientBorsh<Req,Resp>
where
    Req : Debug + Send + Sync + BorshSerialize + 'static,
    Resp : Debug + Send + Sync + BorshDeserialize + 'static,
{
    pub fn new(url : &str) -> Result<Arc<Self>,Error> {
        let rpc = RpcClient::new(url)?;
        Ok(Arc::new(Self {
            rpc, _req_ : PhantomData, _resp_ : PhantomData,
        }))
    }

    pub async fn dispatch(
        self : &Arc<Self>,
        req : Req,
        callback : Arc<Box<(dyn Fn(Result<Resp,Error>) + Sync + Send)>>
    ) -> Result<(),Error> {

        let data = req.try_to_vec().map_err(|_| { Error::BorshSerialize })?;

        self.rpc.dispatch(RpcOps::Borsh as u32, Message::Request(&data), Arc::new(Box::new(move |result| {
            log_trace!("* * * * * Got response: {:?}", result);

            match result {
                Ok(data) => {
                    match data {
                        Some(data) => {
                            match Resp::try_from_slice(&data[..]).map_err(|_| { Error::BorshResponseDeserialize }) {
                                Ok(resp) => callback(Ok(resp)),
                                Err(err) => callback(Err(err)),
                            }
                        },
                        None => {
                            callback(Err(Error::NoDataInSuccessResponse));
                        }
                    }
                },
                Err(err) => {
                    callback(Err(err));
                }
            }
        }))).await?;
        Ok(())
    }

}

#[derive(Debug)]
pub struct ReqMessage<'data> {
    pub id : u64,
    pub op : u32,
    pub data : Option<&'data [u8]>,
    // pub data : Option<&'data [u8]>,
}

impl<'data> TryFrom<&'data Vec<u8>> for ReqMessage<'data> {
    type Error = Error;

    fn try_from(src: &'data Vec<u8>) -> Result<Self, Self::Error> {
        let v : ReqMessage = src[..].try_into()?;
        Ok(v)
    }
}

impl<'data> TryFrom<&'data [u8]> for ReqMessage<'data> {
    type Error = Error;

    fn try_from(src: &'data [u8]) -> Result<Self, Self::Error> {
        if src.len() < size_of::<ReqHeader>() {
            return Err(Error::HeaderSize);
        }

        let header: &ReqHeader = unsafe { std::mem::transmute(&src[0]) };
        let ReqHeader { id, op } = *header;
        let data = if src.len() == size_of::<ReqHeader>() { None } else { Some(&src[size_of::<ReqHeader>()..]) };

        let message = ReqMessage {
            id,
            op,
            data
        };
        
        Ok(message)
    }
}




#[derive(Debug)]
pub struct RespMessage<'data> {
    pub id : u64,
    pub status : u32,
    pub data : Option<&'data [u8]>,
}

impl<'data> RespMessage<'data> {
    pub fn new(id: u64, status: u32, data: Option<&'data [u8]>) -> RespMessage<'data> {
        RespMessage {
            id,
            status,
            data
        }
    }

    pub fn try_to_vec(&self) -> Result<Vec<u8>, Error> {
        match self.data {
            Some(data) => {
                let len = size_of::<RespHeader>() + data.len();
                let mut buffer = Vec::with_capacity(len);
                unsafe { buffer.set_len(len); }
                let header: &mut RespHeader = unsafe { std::mem::transmute(&mut buffer[0]) };
                *header = RespHeader { id : self.id, status : self.status };
                buffer[size_of::<RespHeader>()..].copy_from_slice(&data);
                Ok(buffer)
            },
            None => {
                let mut buffer = Vec::with_capacity(size_of::<RespHeader>());
                unsafe { buffer.set_len(size_of::<RespHeader>()); }
                let header: &mut RespHeader = unsafe { std::mem::transmute(&mut buffer[0]) };
                *header = RespHeader { id : self.id, status : self.status };
                Ok(buffer)

            }
        }
    }
}

impl<'data> TryFrom<&'data [u8]> for RespMessage<'data> {
    type Error = Error;

    fn try_from(src: &'data [u8]) -> Result<Self, Self::Error> {
        if src.len() < size_of::<ReqHeader>() {
            return Err(Error::HeaderSize);
        }

        let header: &RespHeader = unsafe { std::mem::transmute(&src[0]) };
        let RespHeader { id, status } = *header;
        let data = if src.len() == size_of::<RespHeader>() { None } else { Some(&src[size_of::<RespHeader>()..]) };

        let message = RespMessage {
            id,
            status,
            data
        };
        
        Ok(message)
    }
}


#[allow(dead_code)]
static mut RPC : Option<Arc<RpcClientBorsh<TestReq,TestResp>>> = None;
#[cfg(target_arch = "wasm32")]
mod testing {
    #[allow(dead_code)]

    use super::*;
    // use wasm_bindgen::prelude::*;
    #[allow(unused_imports)]
    use workflow_allocator::log::*;
    // use workflow_allocator::wasm::timers;

    #[allow(dead_code)]
    // #[wasm_bindgen(start)]
    pub fn start_websocket() -> Result<(),Error> {

        let rpc = RpcClientBorsh::<TestReq,TestResp>::new("ws://localhost:9090")?;

        // let ws = WebSocket::new("ws://localhost:9090", Settings::default())?;
        // let ws = WebSocket::new("wss://echo.websocket.events")?;

        unsafe { RPC = Some(rpc.clone() )};

        // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
        // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
        // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

        crate::task::spawn(async move {

            loop {


                match rpc.dispatch(TestReq::Second(888),Arc::new(Box::new(move |result : Result<TestResp,Error>|{

                    log_trace!("* * * * * * * * * received response: {:?}", result);

                }))).await {
                    Ok(_) => { log_trace!("dispatch executed successfully"); },
                    Err(err) => { log_trace!("dispatch failed: {:?}", err); },
                }

                async_std::task::sleep(std::time::Duration::from_millis(1000)).await;
            }
        });

        // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
        // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
        // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

        Ok(())
    }

}

