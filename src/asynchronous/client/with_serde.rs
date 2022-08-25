use std::sync::Arc;
use serde::{Serialize,Deserialize,de::DeserializeOwned};
use workflow_log::log_trace;
use super::*;
use super::error::*;
use super::result::Result;

#[derive(Clone)]
pub struct RpcClientSerde 
{
    rpc : RpcClient,
}

impl RpcClientSerde
{
    pub fn new(url : &str) -> Result<Arc<Self>> {
        let rpc = RpcClient::new(url)?;
        Ok(Arc::new(Self {
            rpc,
        }))
    }

    pub async fn connect(&self, block_until_connected:bool) -> Result<()> {
        self.rpc.connect(block_until_connected).await
    }

    // pub async fn dispatch<'resp, Req, Resp>(
    pub async fn dispatch<Req, Resp>(
        self : &Arc<Self>,
        req : Req,
        callback : Arc<Box<(dyn Fn(Result<Resp>) + Sync + Send)>>
    ) -> Result<()> 
    where
        Req : Serialize + Send + Sync + 'static,
        // Resp : Deserialize<'resp> + Send + Sync + 'static,
        Resp : DeserializeOwned + Send + Sync + 'static,
    {

        let json = serde_json::to_string(&req)?;
        //.map_err(|e:serde_json::Error|e.into())?; 

        self.rpc.call(RpcOps::Serde as u32, Message::Request(&json.as_bytes()), Arc::new(Box::new(move |result| {
            log_trace!("* * * * * Got response: {:?}", result);

            match result {
                Ok(data) => {
                    match data {
                        Some(data) => {
                            match String::from_utf8(data.to_vec()) {
                                Ok(utf8) => {
                                    // TODO use Deserialize instead of DeserializeOwned
                                    match serde_json::from_str(&utf8) {
                                            Ok(resp) => { callback(Ok(resp)) },
                                            Err(e) => callback(Err(e.into())),
                                    }
                                },
                                Err(e) => {
                                    callback(Err(e.utf8_error().into()));
                                }
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
