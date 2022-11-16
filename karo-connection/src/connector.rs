use anyhow::Result;
use async_trait::async_trait;
use tokio::net::UnixStream;

#[async_trait]
pub trait Connector {
    async fn connect(&self) -> Result<UnixStream>;
}
