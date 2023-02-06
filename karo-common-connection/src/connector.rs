use anyhow::Result;
use async_trait::async_trait;
use tokio::net::UnixStream;

use crate::writer::Writer;

/// Connector, which connects to a peer.
/// Used not only for the initial connection, but also to reconnect if connection closed
#[async_trait]
pub trait Connector: Send + Sync {
    async fn connect(&self) -> Result<UnixStream>;

    /// Connected writer handle to send post-connection messages
    async fn on_connected(&self, _: &mut Writer) -> Result<()> {
        Ok(())
    }
}
