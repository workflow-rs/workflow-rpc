use borsh::{BorshSerialize,BorshDeserialize};

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub enum RpcResponseError {
    NoData,
    NonBorshRequest,
    ReqDeserialize,
    RespSerialize,
    Data(Vec<u8>),
}
