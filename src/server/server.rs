use std::fmt::Debug;
use std::sync::Arc;
use async_trait::async_trait;
use workflow_websocket::server::WebSocketHandler;
use crate::message::*;
use crate::error::RpcResponseError;
use tokio::sync::mpsc::*;
use workflow_log::*;
use workflow_websocket::server::WebSocketServer;
use tungstenite::{Message, Result};
use borsh::BorshSerialize;

#[async_trait]
pub trait RpcHandler<Ops> : Send + Sync + 'static
where
    Ops : Debug + Send + Sync + 'static
{
    async fn handle_request(self : Arc<Self>, op : Ops, data : Option<&[u8]>) -> Result<Option<Vec<u8>>, RpcResponseError>;
}

#[derive(Clone)]
pub struct RpcWebSocketHandler<Ops>
where
    Ops: Debug + Send + Sync + TryFrom<u32> + 'static
{
    rpc_handler : Arc<dyn RpcHandler<Ops>>
}

impl<Ops> RpcWebSocketHandler<Ops>
where
    Ops: Debug + Send + Sync + TryFrom<u32> + 'static
{
    pub fn new(rpc_handler : Arc<dyn RpcHandler<Ops>>) -> Self {
        Self {
            rpc_handler
        }
    }
}

#[async_trait]
impl<Ops> WebSocketHandler for RpcWebSocketHandler<Ops>
where 
    Ops: Debug + Clone + Send + Sync + TryFrom<u32> + 'static,
    <Ops as TryFrom<u32>>::Error: Sync + Send + 'static
{
    async fn handle_message(self : &Arc<Self>, msg : Message, sink : UnboundedSender<tungstenite::Message>) {

        let data = &msg.into_data();
        let req : ReqMessage = data.try_into().expect("invalid message!");

        let op = Ops::try_from(req.op); 
        match op {
            Ok(op) => {
                let result = self.rpc_handler.clone().handle_request(op,req.data).await;
                match result {
                    Ok(data) => {
                        if let Ok(msg) = RespMessage::new(req.id, 0, data.as_deref()).try_to_vec() {
                            match sink.send(msg.into()) {
                                Ok(_) => {},
                                Err(e) => { log_trace!("Sink error: {:?}", e); }
                            }
                        }
                    },
                    Err(err) => {
                        if let Ok(err) = err.try_to_vec() {
                            if let Ok(msg) = RespMessage::new(req.id, 1, Some(&err)).try_to_vec() {
                                match sink.send(msg.into()) {
                                    Ok(_) => {},
                                    Err(e) => { log_trace!("Sink error: {:?}", e); }
                                }
                            }
                        }
                    }
                }
            },
            Err(_) => {
                log_error!("invalid request opcode {}", req.op);                
            }
        }
    }
}

pub struct RpcServer<Ops>
where 
    Ops : Debug + Clone + Send + Sync  + TryFrom<u32> + 'static,
    <Ops as TryFrom<u32>>::Error: Sync + Send + 'static
{
    ws_server : Arc<WebSocketServer<RpcWebSocketHandler<Ops>>>,
}

impl<Ops> RpcServer<Ops>
where 
    Ops : Debug + Clone + Send + Sync + TryFrom<u32> + 'static,
    <Ops as TryFrom<u32>>::Error: Sync + Send + 'static
{
    pub fn new(rpc_handler : Arc<dyn RpcHandler<Ops>>) -> Arc<RpcServer<Ops>> {
        let ws_handler = Arc::new(RpcWebSocketHandler::<Ops>::new(rpc_handler));
        let ws_server = WebSocketServer::new(ws_handler);
        Arc::new(RpcServer { ws_server })
    }

    pub async fn listen(self : &Arc<Self>, addr : &str) -> Result<()> {
        self.ws_server.listen(addr).await
    }
}
