use anyhow::Result;
use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[async_trait]
pub trait Connector<S: AsyncReadExt + AsyncWriteExt> {
    async fn connect(&self) -> Result<S>;
}
