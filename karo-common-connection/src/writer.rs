use anyhow::{Context, Result};
use bson::Bson;
use tokio::{net::UnixStream, sync::mpsc::Sender};

/// Writer into a socket.
/// Can be cloned to simultaniously send data to the peer
#[derive(Clone)]
pub struct Writer {
    /// Sender to send a message to a socket loop.
    /// Failing to send into the sender means connection closed
    sender: Sender<(Bson, Option<UnixStream>)>,
}

impl Writer {
    pub fn new(sender: Sender<(Bson, Option<UnixStream>)>) -> Self {
        Self { sender }
    }

    /// Write outgoing Bson message
    pub async fn write_bson(&mut self, message: &Bson) -> Result<()> {
        // If failed to write into a socket. we loop and let reader to reconnect
        self.sender
            .send((message.clone(), None))
            .await
            .context("Connection closed")
    }

    /// Write FD following data
    pub async fn write_bson_fd(&mut self, message: &Bson, stream: UnixStream) -> Result<()> {
        self.sender
            .send((message.clone(), Some(stream)))
            .await
            .context("Connection closed")
    }
}
