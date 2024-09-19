mod handle;
use std::net::SocketAddr;

use anyhow::Result;
use async_trait::async_trait;
use dialoguer::Error as DialoguerError;
pub use handle::{SessionHandle, WarpgateServerHandle};
use warpgate_common::Target;

#[derive(Debug, thiserror::Error)]
pub enum TargetTestError {
    #[error("unreachable")]
    Unreachable,
    #[error("authentication failed")]
    AuthenticationError,
    #[error("connection error: {0}")]
    ConnectionError(String),
    #[error("misconfigured: {0}")]
    Misconfigured(String),
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("dialoguer error: {0}")]
    DialoguerError(DialoguerError),
}

#[async_trait]
pub trait ProtocolServer {
    async fn run(self, address: SocketAddr) -> Result<()>;
    async fn test_target(&self, target: Target) -> Result<(), TargetTestError>;
}
