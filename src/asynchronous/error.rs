use borsh::{BorshSerialize,BorshDeserialize};

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub enum RpcResponseError {
    NoData,
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