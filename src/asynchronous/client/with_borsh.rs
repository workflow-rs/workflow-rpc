use std::sync::Arc;
// use std::marker::PhantomData;
// use std::fmt::Debug;
use borsh::{BorshSerialize,BorshDeserialize};
use workflow_log::log_trace;
use workflow_core::channel::oneshot;
use super::*;
use super::error::Error;
use super::result::Result;


// use crate::asynchronous::client::RpcClient;
// use crate::client::error::Error;
// use crate::client::result::Result;
// use crate::message::Message;
// use crate::ops::RpcOps;

#[derive(Clone)]
pub struct RpcClientBorsh //<Req,Resp> 
// where
//     Req : Debug + Send + Sync + BorshSerialize + 'static,
//     Resp : Debug + Send + Sync + BorshDeserialize + 'static,
{
    rpc : RpcClient,
    // _req_ : PhantomData<Req>,
    // _resp_ : PhantomData<Resp>,
}

// impl<Req,Resp> RpcClientBorsh<Req,Resp>
impl RpcClientBorsh
// where
//     Req : Debug + Send + Sync + BorshSerialize + 'static,
//     Resp : Debug + Send + Sync + BorshDeserialize + 'static,
{
    pub fn new(url : &str) -> Result<Self> {
        let rpc = RpcClient::new(url)?;
        Ok(Self {
            rpc,// _req_ : PhantomData, _resp_ : PhantomData,
        })
    }

    pub async fn connect(&self, block_until_connected:bool) -> Result<()> {
        self.rpc.connect(block_until_connected).await
    }

    fn deser<Resp>(intake : Result<Option<&[u8]>>) -> Result<Resp> 
    where
        Resp : BorshDeserialize + Send + Sync + 'static,

    {
        match intake {
            Ok(data) => {
                match data {
                    Some(data) => {
                        Ok(Resp::try_from_slice(&data[..]).map_err(|_| { Error::BorshResponseDeserialize })?)
                        // Ok(Resp::try_from_slice(&data[..]).map_err(|_| { Error::BorshResponseDeserialize })?) 
                    },
                    None => {
                        Err(Error::NoDataInSuccessResponse)
                    }
                }
            },
            Err(err) => {
                Err(err)
            }
        }
    }

    fn deser_data<Resp>(intake : Option<&[u8]>) -> Result<Resp>
    where
        Resp : BorshDeserialize + Send + Sync + 'static,

    {
        match intake {
            Some(data) => {
                Ok(Resp::try_from_slice(&data[..]).map_err(|_| { Error::BorshResponseDeserialize })?)
                // Ok(Resp::try_from_slice(&data[..]).map_err(|_| { Error::BorshResponseDeserialize })?) 
            },
            None => {
                Err(Error::NoDataInSuccessResponse)
            }
            
        }
    }

    pub async fn call_with_callback<Req,Resp>(
        &self,
        req : Req,
        callback : Arc<Box<(dyn Fn(Result<Resp>) + Sync + Send)>>
    ) -> Result<()> 
    where
        Req : BorshSerialize + Send + Sync + 'static,
        Resp : BorshDeserialize + Send + Sync + 'static,
    {
        let data = req.try_to_vec().map_err(|_| { Error::BorshSerialize })?;
        self.rpc.call_with_callback(RpcOps::Borsh as u32, Message::Request(&data), Arc::new(Box::new(move |result| {
            log_trace!("* * * * * Got response: {:?}", result);
            let resp : Result<Resp> = Self::deser(result);
            callback(resp);
        }))).await?;
        Ok(())
    }

    pub async fn call<Req,Resp>(
        &self,
        req : Req,
        // callback : Arc<Box<(dyn Fn(Result<Resp>) + Sync + Send)>>
    ) -> std::result::Result<Resp,Error> 
    where
        Req : BorshSerialize + Send + Sync + 'static,
        Resp : BorshDeserialize + Send + Sync + 'static,
    {
        let (sender,receiver) = oneshot();
        let data = req.try_to_vec().map_err(|_| { Error::BorshSerialize })?;
        self.rpc.call_with_callback(RpcOps::Borsh as u32, Message::Request(&data), Arc::new(Box::new(move |result| {

            log_trace!("* * * * * Got response: {:?}", result);
            let resp = match result {
                Ok(resp) => {
                    Ok(Self::deser_data::<Result<Resp>>(resp))
                },
                Err(err) => {
                    Err(err.into())
                }
            };
            // let resp : Result<Resp> = Self::deser(result);
            sender.try_send(resp).expect("RpcCLientBorsh::call() response send channel error");
            // callback(resp);
        }))).await?;

        receiver.recv().await.expect("RpcCLientBorsh::call() response recv channel error").unwrap().unwrap()
        // Ok(())
    }

}
