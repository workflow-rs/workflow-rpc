use std::net::SocketAddr;
use std::sync::Arc;
use async_trait::async_trait;
use workflow_websocket::server::WebSocketHandler;
use crate::asynchronous::message::*;
use crate::asynchronous::error::RpcResponseError;
use tokio::sync::mpsc::*;
use workflow_log::*;
use workflow_websocket::server::{
    WebSocketServer, Result as WebSocketResult
};
// use tungstenite::{Message, Result};
use tungstenite::Message;
use borsh::BorshSerialize;


pub fn result<Resp>(resp:Resp) -> Result<Option<Vec<u8>>,RpcResponseError>
where Resp : BorshSerialize {
    let data = resp.try_to_vec().map_err(|_|RpcResponseError::RespSerialize)?;
    Ok(Some(data))
}

pub struct RpcContext {
    pub peer : SocketAddr,
}


#[async_trait]
pub trait RpcHandler<Ops> : Send + Sync + 'static
where
    Ops : Send + Sync + 'static
{
    async fn handle_request(self : Arc<Self>, op : Ops, data : Option<&[u8]>) -> Result<Option<Vec<u8>>, RpcResponseError>;
}

#[derive(Clone)]
pub struct RpcWebSocketHandler<Ops>
where
    Ops: Send + Sync + TryFrom<u32> + 'static
{
    rpc_handler : Arc<dyn RpcHandler<Ops>>
}

impl<Ops> RpcWebSocketHandler<Ops>
where
    Ops: Send + Sync + TryFrom<u32> + 'static
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
    Ops: Send + Sync + TryFrom<u32> + 'static,
    <Ops as TryFrom<u32>>::Error: Sync + Send + 'static
{
    type Context = Arc<RpcContext>;

    async fn connect(self : &Arc<Self>, peer: SocketAddr) -> WebSocketResult<Self::Context> {
        let ctx = RpcContext { peer };
        Ok(Arc::new(ctx))
    }

    async fn handshake(self : &Arc<Self>, _ctx : &Self::Context, _msg : Message, _sink : &UnboundedSender<tungstenite::Message>) -> WebSocketResult<()> {
        Ok(())
    }

    async fn message(self : &Arc<Self>, _ctx : &Self::Context, msg : Message, sink : &UnboundedSender<tungstenite::Message>) -> WebSocketResult<()> {

        let data = &msg.into_data();
        let req : ReqMessage = data.try_into().expect("invalid message!");

        let op = Ops::try_from(req.op); 
        match op {
            Ok(op) => {
                // log_trace!("receiving message {:?}", req.data);
                let result = self.rpc_handler.clone().handle_request(op,req.data).await;
                match result {
                    Ok(data) => {
                        // log_trace!("sending response {:?}",data);
                        if let Ok(msg) = RespMessage::new(req.id, 0, data.as_deref()).try_to_vec() {
                            // println!("FULL RESPONSE: {:?}",msg);
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

        Ok(())
    }
}

pub struct RpcServer<Ops>
where 
    Ops : Send + Sync  + TryFrom<u32> + 'static,
    <Ops as TryFrom<u32>>::Error: Sync + Send + 'static
{
    ws_server : Arc<WebSocketServer<RpcWebSocketHandler<Ops>>>,
}

impl<Ops> RpcServer<Ops>
where 
    Ops : Send + Sync + TryFrom<u32> + 'static,
    <Ops as TryFrom<u32>>::Error: Sync + Send + 'static
{
    pub fn new(rpc_handler : Arc<dyn RpcHandler<Ops>>) -> Arc<RpcServer<Ops>> {
        let ws_handler = Arc::new(RpcWebSocketHandler::<Ops>::new(rpc_handler));
        let ws_server = WebSocketServer::new(ws_handler);
        Arc::new(RpcServer { ws_server })
    }

    pub async fn listen(self : &Arc<Self>, addr : &str) -> WebSocketResult<()> {
        Ok(self.ws_server.listen(addr).await?)
    }
}
