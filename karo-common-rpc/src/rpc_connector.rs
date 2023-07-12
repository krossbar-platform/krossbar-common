use anyhow::Result;
use async_trait::async_trait;
use tokio::net::UnixStream;

use crate::rpc_sender::RpcSender;

/// Connector, which connects to a peer.
/// Used not only for the initial connection, but also to reconnect if connection closed
#[async_trait]
pub trait RpcConnector: Send + Sync {
    async fn connect(&self) -> Result<UnixStream>;

    /// Connected writer handle to send post-connection messages
    async fn on_connected(&self, _: &mut RpcSender) -> Result<()> {
        Ok(())
    }
}
