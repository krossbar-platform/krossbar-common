use anyhow::{Context, Result};
use bytes::Bytes;
use tokio::sync::mpsc::Sender;

#[derive(Clone)]
pub struct Writer {
    sender: Sender<Bytes>,
}

impl Writer {
    pub fn new(sender: Sender<Bytes>) -> Self {
        Self { sender }
    }

    pub async fn write(&mut self, bytes: Bytes) -> Result<()> {
        self.sender
            .send(bytes)
            .await
            .context("Failed to write outgoing message. Connection closed")
    }
}
