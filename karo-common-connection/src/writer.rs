use anyhow::{Context, Result};
use bson::Bson;
use bytes::Bytes;
use tokio::sync::mpsc::Sender;

/// Writer into a socket.
/// Can be cloned to simultaniously send data to the peer
#[derive(Clone)]
pub struct Writer {
    /// Sender to send a message to a socket loop
    sender: Sender<Bytes>,
}

impl Writer {
    pub fn new(sender: Sender<Bytes>) -> Self {
        Self { sender }
    }

    /// Write outgoing Bson message
    pub async fn write_bson(&mut self, message: &Bson) -> Result<()> {
        let bytes = bson::to_raw_document_buf(message)
            .map(|data| data.into_bytes())
            .context("Failed to serialize incoming Bson")?;

        self.write(Bytes::from(bytes))
            .await
            .context("Failed to write outgoing message into the socket")
    }

    /// Write raw Bson data
    async fn write(&mut self, bytes: Bytes) -> Result<()> {
        self.sender
            .send(bytes)
            .await
            .context("Failed to write outgoing message. Connection closed")
    }
}
