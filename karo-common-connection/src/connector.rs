use anyhow::Result;
use async_trait::async_trait;
use tokio::net::UnixStream;

/// Connector, which connects to a peer.
/// Used not only for the initial connection, but also to reconnect if connection closed
#[async_trait]
pub trait Connector: Send + Sync {
    async fn connect(&self) -> Result<UnixStream>;
}
