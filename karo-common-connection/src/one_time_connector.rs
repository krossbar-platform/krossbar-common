use anyhow::{anyhow, Result};
use async_trait::async_trait;
use tokio::{net::UnixStream, sync::Mutex};

use crate::connector::Connector;

/// Connector, which is allows first connection, but later refuses
/// reconnection attempts
pub struct OneTimeConnector {
    stream: Mutex<Option<UnixStream>>,
}

impl OneTimeConnector {
    pub fn new(stream: UnixStream) -> Self {
        Self {
            stream: Mutex::new(Some(stream)),
        }
    }
}

#[async_trait]
impl Connector for OneTimeConnector {
    async fn connect(&self) -> Result<UnixStream> {
        if let Some(stream) = self.stream.lock().await.take() {
            Ok(stream)
        } else {
            Err(anyhow!("Does not allowed to reconnect"))
        }
    }
}
