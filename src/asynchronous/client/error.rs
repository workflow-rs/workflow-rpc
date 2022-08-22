use thiserror::Error;
use workflow_websocket::client::error::Error as WebSocketError;
use wasm_bindgen::JsValue;
use crate::asynchronous::error::RpcResponseError;

#[derive(Error, Debug)]
pub enum Error {

    /// Underlying WebSocket error
    #[error("WebSocket error: {0}")]
    WebSocketError(#[from] WebSocketError),
    /// RPC call timeout
    #[error("RPC request timeout")]
    Timeout,
    /// Unable to send shutdown message to receiver
    #[error("Receiver ctl failure")]
    ReceiverCtl,
    /// Received message is smaller than the minimum header size
    #[error("Invalid header size")]
    HeaderSize,
    /// RPC call succeeded but no data was received in success response
    #[error("RPC: no data in success response")]
    NoDataInSuccessResponse,
    /// RPC call failed but no data was received in error response
    #[error("RPC: no data in error response")]
    NoDataInErrorResponse,
    /// Unable to deserialize response data
    #[error("RPC: error deserializing response data")]
    ErrorDeserializingResponseData,
    /// Response produced an unknown status code
    #[error("RPC: status code {0}")]
    StatusCode(u32),
    /// RPC call executed successfully but produced an error response
    #[error("RPC: response error {0:?}")]
    RpcCall(RpcResponseError),
    /// Unable to serialize borsh data    
    #[error("RPC: borsh serialization error")]
    BorshSerialize,
    /// Unable to deserialize borsh data
    #[error("RPC: borsh deserialization error")]
    BorshDeserialize,
    /// RPC call succeeded, but error occurred deserializing borsh response
    #[error("RPC: borsh error deserializing response")]
    BorshResponseDeserialize,
}

/// Transform Error into JsValue containing the error message
impl Into<JsValue> for Error {
    fn into(self) -> JsValue {
        JsValue::from(format!("{}",self).to_string())
    }
}
