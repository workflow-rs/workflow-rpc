use std::sync::Arc;
use async_trait::async_trait;
use borsh::BorshSerialize;
use crate::asynchronous::server::RpcHandler;
use crate::asynchronous::error::RpcResponseError;


#[async_trait]
pub trait RpcHandlerBorsh<Ops> : Send + Sync + 'static
where
    Ops : Send + Sync + 'static,
{
    async fn handle_request(self : Arc<Self>,  op : Ops, data : &[u8]) -> Result<Vec<u8>, RpcResponseError>;
}

#[derive(Clone)]
pub struct RpcHandlerBorshAdaptor<Ops> 
where
    Ops : Send + Sync + 'static,
{
    handler : Arc<dyn RpcHandlerBorsh<Ops>>,
}

impl<Ops> RpcHandlerBorshAdaptor<Ops>
where
    Ops : Send + Sync + 'static,
{
    pub fn new(handler: Arc<dyn RpcHandlerBorsh<Ops>>) -> Self {
        Self { handler }
    }
}


#[async_trait]
impl<Ops> RpcHandler<Ops> for RpcHandlerBorshAdaptor<Ops> 
where 
    Ops : Send + Sync + 'static,
{
    async fn handle_request(self : Arc<Self>, op: Ops, data: Option<&[u8]>) -> Result<Option<Vec<u8>>, RpcResponseError> {

        match data {
            Some(data) => {
                let response = self.handler.clone().handle_request(op,data).await;
                match response {
                    Ok(data) => Ok(Some(data.try_to_vec().map_err(|_|{ RpcResponseError::RespSerialize })?)),
                    Err(err) => Err(err),
                }
            },
            None => {
                Err(RpcResponseError::NoData)
            },
        }
    }
}
