use std::sync::PoisonError;
use borsh::{BorshSerialize,BorshDeserialize};

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub enum RpcResponseError {
    NoData,
    PoisonError,
    NonBorshRequest,
    NonSerdeRequest,
    ReqDeserialize,
    RespSerialize,
    Data(Vec<u8>),
    Text(String),
}

impl From<std::io::Error> for RpcResponseError {
    fn from(_err: std::io::Error) -> Self {
        RpcResponseError::RespSerialize
    }
}

impl<T> From<PoisonError<T>> for RpcResponseError {
    fn from(_error: PoisonError<T>) -> RpcResponseError {
        RpcResponseError::PoisonError
    }
}
