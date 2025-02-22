mod handle;

use std::future::Future;

use anyhow::Result;
use dialoguer::Error as DialoguerError;
pub use handle::{SessionHandle, WarpgateServerHandle};
use warpgate_common::{ListenEndpoint, Target};

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
}

impl From<DialoguerError> for TargetTestError {
    fn from(err: DialoguerError) -> Self {
        TargetTestError::ConnectionError(format!("Dialoguer error: {}", err))
    }
}

pub trait ProtocolServer {
    fn run(self, address: ListenEndpoint) -> impl Future<Output = Result<()>> + Send;
    fn test_target(
        &self,
        target: Target,
    ) -> impl Future<Output = Result<(), TargetTestError>> + Send;
}
