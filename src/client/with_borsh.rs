use std::sync::Arc;
use std::marker::PhantomData;
use std::fmt::Debug;
use borsh::{BorshSerialize,BorshDeserialize};
use workflow_log::log_trace;
use crate::client::RpcClient;
use crate::client::error::Error;
use crate::client::result::Result;
use crate::message::Message;
use crate::ops::RpcOps;

#[derive(Clone)]
pub struct RpcClientBorsh<Req,Resp> 
where
    Req : Debug + Send + Sync + BorshSerialize + 'static,
    Resp : Debug + Send + Sync + BorshDeserialize + 'static,
{
    rpc : RpcClient,
    _req_ : PhantomData<Req>,
    _resp_ : PhantomData<Resp>,
}

impl<Req,Resp> RpcClientBorsh<Req,Resp>
where
    Req : Debug + Send + Sync + BorshSerialize + 'static,
    Resp : Debug + Send + Sync + BorshDeserialize + 'static,
{
    pub fn new(url : &str) -> Result<Arc<Self>> {
        let rpc = RpcClient::new(url)?;
        Ok(Arc::new(Self {
            rpc, _req_ : PhantomData, _resp_ : PhantomData,
        }))
    }

    pub async fn connect(&self, block_until_connected:bool) -> Result<()> {
        self.rpc.connect(block_until_connected).await
    }

    pub async fn dispatch(
        self : &Arc<Self>,
        req : Req,
        callback : Arc<Box<(dyn Fn(Result<Resp>) + Sync + Send)>>
    ) -> Result<()> 
    where
        Req : BorshSerialize,
        Resp : BorshDeserialize,
    {

        let data = req.try_to_vec().map_err(|_| { Error::BorshSerialize })?;

        self.rpc.call(RpcOps::Borsh as u32, Message::Request(&data), Arc::new(Box::new(move |result| {
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
