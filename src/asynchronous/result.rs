use super::error::RpcResponseError;
pub type RpcResult = std::result::Result<Vec<u8>, RpcResponseError>;