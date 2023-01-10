use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use tokio::{net::UnixStream, time::sleep};

use karo_common_connection::connector::Connector;

const SLEEP_DURATION: Duration = Duration::from_millis(10);

pub struct SimpleConnector {
    socket_path: String,
}

impl SimpleConnector {
    pub fn new(socket_path: &str) -> Self {
        Self {
            socket_path: socket_path.into(),
        }
    }
}

#[async_trait]
impl Connector<UnixStream> for SimpleConnector {
    async fn connect(&self) -> Result<UnixStream> {
        loop {
            if let Ok(stream) = UnixStream::connect(&self.socket_path).await {
                return Ok(stream);
            }

            sleep(SLEEP_DURATION).await
        }
    }
}
