use thiserror::Error;
use workflow_websocket::client::error::Error as WebSocketError;
use wasm_bindgen::JsValue;
use crate::error::RpcResponseError;

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
