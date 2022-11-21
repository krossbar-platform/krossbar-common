use anyhow::Result;
use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Connector, which connects to a peer.
/// Used not only for the initial connection, but also to reconnect if connection closed
#[async_trait]
pub trait Connector<S: AsyncReadExt + AsyncWriteExt> {
    async fn connect(&self) -> Result<S>;
}
