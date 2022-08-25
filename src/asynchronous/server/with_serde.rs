// use std::fmt::Debug;
// use std::sync::Arc;
// use async_trait::async_trait;
// use serde::{Serialize,Deserialize};
// use crate::asynchronous::server::RpcHandler;
// use crate::asynchronous::ops::RpcOps;
// use crate::asynchronous::error::RpcResponseError;

// #[async_trait]
// pub trait RpcHandlerSerde //<Req,Resp> : Send + Sync + 'static
// // where
// //     Req : Send + Sync + 'static,
// //     Resp : Send + Sync + 'static,
// {
//     async fn handle_request(self : Arc<Self>, request : Req) -> Result<Resp, RpcResponseError>;
// }

// #[derive(Clone)]
// pub struct RpcHandlerSerdeAdaptor<Req,Resp> 
// // where 
// //     Req : Send + Sync + 'static,
// //     Resp : Send + Sync + 'static,
// {
//     handler : Arc<dyn RpcHandlerSerde<Req,Resp>>,
// }

// impl<Req,Resp> RpcHandlerSerdeAdaptor<Req,Resp>
// // where
// //     Req : Send + Sync + 'static,
// //     Resp : Send + Sync + 'static,
// {
//     pub fn new(handler: Arc<dyn RpcHandlerSerde<Req,Resp>>) -> Self {
//         Self { handler }
//     }
// }


// #[async_trait]
// impl<Req,Resp> RpcHandler<RpcOps> for RpcHandlerSerdeAdaptor<Req,Resp> 
// where 
//     // Req : Send + Sync + 'static,
//     // Resp : Send + Sync + 'static,
// {
//     async fn handle_request(self : Arc<Self>, ops: RpcOps, data: Option<&[u8]>) -> Result<Resp, RpcResponseError> {

//         // match data {
//         //     Some(data) if ops == RpcOps::Serde => {
//         //         let req = Req::try_from_slice(data).map_err(|_| { RpcResponseError::ReqDeserialize})?;

//         //         let response = self.handler.clone().handle_request(req).await;
//         //         match response {
//         //             Ok(data) => Ok(Some(data.try_to_vec().map_err(|_|{ RpcResponseError::RespSerialize })?)),
//         //             Err(err) => Err(err),
//         //         }
//         //     },
//         //     None => {
//         //         Err(RpcResponseError::NoData)
//         //     },
//         //     _ => {
//         //         Err(RpcResponseError::NonSerdeRequest)
//         //     }
//         // }

//         Ok(None)
//     }
// }
