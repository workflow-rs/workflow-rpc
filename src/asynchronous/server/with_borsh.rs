use std::fmt::Debug;
use std::sync::Arc;
use async_trait::async_trait;
use borsh::{BorshSerialize,BorshDeserialize};
use crate::asynchronous::server::RpcHandler;
use crate::asynchronous::ops::RpcOps;
use crate::asynchronous::error::RpcResponseError;

#[async_trait]
pub trait RpcHandlerBorsh<Req,Resp> : Send + Sync + 'static
where
    Req : Debug + Send + Sync + BorshDeserialize + 'static,
    Resp : Debug + Send + Sync + BorshSerialize + 'static,
{
    async fn handle_request(self : Arc<Self>, request : Req) -> Result<Resp, RpcResponseError>;
}

#[derive(Clone)]
pub struct RpcHandlerBorshAdaptor<Req,Resp> 
where 
    Req : Debug + Send + Sync + BorshDeserialize + 'static,
    Resp : Debug + Send + Sync + BorshSerialize + 'static,
{
    handler : Arc<dyn RpcHandlerBorsh<Req,Resp>>,
}

impl<Req,Resp> RpcHandlerBorshAdaptor<Req,Resp>
where
    Req : Debug + Send + Sync + BorshDeserialize + 'static,
    Resp : Debug + Send + Sync + BorshSerialize + 'static,
{
    pub fn new(handler: Arc<dyn RpcHandlerBorsh<Req,Resp>>) -> Self {
        Self { handler }
    }
}


#[async_trait]
impl<Req,Resp> RpcHandler<RpcOps> for RpcHandlerBorshAdaptor<Req,Resp> 
where 
    Req : Debug + Send + Sync + BorshDeserialize + 'static,
    Resp : Debug + Send + Sync + BorshSerialize + 'static,
{
    async fn handle_request(self : Arc<Self>, ops: RpcOps, data: Option<&[u8]>) -> Result<Option<Vec<u8>>, RpcResponseError> {

        match data {
            Some(data) if ops == RpcOps::Borsh => {
                let req = Req::try_from_slice(data).map_err(|_| { RpcResponseError::ReqDeserialize})?;

                let response = self.handler.clone().handle_request(req).await;
                match response {
                    Ok(data) => Ok(Some(data.try_to_vec().map_err(|_|{ RpcResponseError::RespSerialize })?)),
                    Err(err) => Err(err),
                }
            },
            None => {
                Err(RpcResponseError::NoData)
            },
            _ => {
                Err(RpcResponseError::NonBorshRequest)
            }
        }
    }
}
