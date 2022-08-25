use std::fmt::Display;

use thiserror::Error;
use workflow_websocket::client::error::Error as WebSocketError;
use wasm_bindgen::JsValue;
use workflow_core::channel::{RecvError,SendError};
use crate::asynchronous::error::RpcResponseError;
use serde::*;
// use borsh::*;

#[derive(Error, Debug)] // , BorshSerialize, BorshDeserialize)]
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
    /// Unable to serialize serde data    
    #[error("RPC: serde serialization error")]
    SerdeSerialize(String), //#[from] dyn serde::de::Error),
    /// Unable to deserialize serde data
    #[error("RPC: serde deserialization error")]
    SerdeDeserialize(String),
    /// RPC call succeeded, but error occurred deserializing borsh response
    #[error("RPC: borsh error deserializing response")]
    BorshResponseDeserialize,

    #[error("RPC: channel receive error")]
    ChannelRecvError,

    #[error("RPC: channel send error")]
    ChannelSendError,

    #[error("Utf8 error: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("SerdeJSON error: {0}")]
    SerdeJSON(#[from] serde_json::Error),
}

/// Transform Error into JsValue containing the error message
impl Into<JsValue> for Error {
    fn into(self) -> JsValue {
        JsValue::from(format!("{}",self).to_string())
    }
}

impl From<RecvError> for Error {
    fn from(_: RecvError) -> Self {
        Error::ChannelRecvError
    }
}

impl<T> From<SendError<T>> for Error {
    fn from(_: SendError<T>) -> Self {
        Error::ChannelSendError
    }
}

// impl ser::Error for Error {
//     fn custom<T: Display>(msg: T) -> Self {
//         Error::SerdeSerialize(msg.to_string())
//     }
// }

// impl de::Error for Error {
//     fn custom<T: Display>(msg: T) -> Self {
//         Error::SerdeDeserialize(msg.to_string())
//     }
// }


// impl From<dyn serde::ser::Error> for Error
// where dyn serde::ser::Error: Sized
// {
//     fn from(e: dyn serde::ser::Error) -> Error {
//         Error::SerdeSerialize(e.to_string())
//     }
//     // fn custom<T: Display>(msg: T) -> Self {
//     //     Error::Message(msg.to_string())
//     // }
// }

// impl From<dyn serde::de::Error> for Error {
//     fn from(e: dyn serde::de::Error) -> Error {
//         Error::SerdeDeserialize(e.to_string())
//     }
//     // fn custom<T: Display>(msg: T) -> Self {
//     //     Error::Message(msg.to_string())
//     // }
// }

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Error {
        Error::SerdeDeserialize(msg.to_string())
    }
}

// impl From<serde_json::Error> for Error {
//     fn from(err: serde_json::Error) -> Error {
//         Error::SerdeDeserialize(err.to_string())
//     }
// }

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Error {
        Error::SerdeSerialize(msg.to_string())
    }
}

// impl From<std::str::Utf8Error> for Error {
//     fn from(e: std::str::Utf8Error) -> Error {
//         Error::Utf8Error(e)
//     }
// }
