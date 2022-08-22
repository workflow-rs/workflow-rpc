use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {

    #[error("WebSocket error: {0}")]
    WebSocketError(#[from] workflow_websocket::server::Error),

}
