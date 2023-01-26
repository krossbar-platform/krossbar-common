use std::{ops::DerefMut, os::unix::prelude::IntoRawFd, sync::Arc};

use anyhow::{anyhow, Context, Result};
use bson::Bson;
use bytes::Bytes;
use tokio::{
    io::AsyncWriteExt,
    net::{unix::OwnedWriteHalf, UnixStream},
    sync::Mutex,
};
use tokio_send_fd::SendFd;

/// Writer into a socket.
/// Can be cloned to simultaniously send data to the peer
#[derive(Clone)]
pub struct Writer {
    /// Sender to send a message to a socket loop.
    /// Having None means we've closed connection
    send_stream: Arc<Mutex<Option<OwnedWriteHalf>>>,
}

impl Writer {
    pub fn new(send_stream: OwnedWriteHalf) -> Self {
        Self {
            send_stream: Arc::new(Mutex::new(Some(send_stream))),
        }
    }

    /// Write outgoing Bson message
    pub async fn write_bson(&mut self, message: &Bson) -> Result<()> {
        let bytes = Bytes::from(
            bson::to_raw_document_buf(message)
                .map(|data| data.into_bytes())
                .context("Failed to serialize incoming Bson")?,
        );

        // If failed to write into a socket. we loop and let reader to reconnect
        loop {
            match self.send_stream.lock().await.deref_mut() {
                Some(socket) => {
                    // Return if succeeded or try again after reconnection
                    if socket.write(&bytes).await.is_ok() {
                        return Ok(());
                    }
                }
                None => return Err(anyhow!("Connection closed")),
            }
        }
    }

    /// Write FD following data
    pub async fn write_bson_fd(&mut self, message: &Bson, stream: UnixStream) -> Result<()> {
        let bytes = Bytes::from(
            bson::to_raw_document_buf(message)
                .map(|data| data.into_bytes())
                .context("Failed to serialize incoming Bson")?,
        );

        let os_stream = stream.into_std().unwrap();
        let fd = os_stream.into_raw_fd();

        loop {
            match self.send_stream.lock().await.deref_mut() {
                Some(socket) => {
                    if socket.write(&bytes).await.is_ok()
                        && socket.as_ref().send_fd(fd).await.is_ok()
                    // Return if succeeded or try again after reconnection
                    {
                        return Ok(());
                    }
                }

                None => return Err(anyhow!("Connection closed")),
            }
        }
    }

    /// Replace socket half with a new handle if reconnected succesfully, or None if failed
    /// to reconnect, which means that connection closed
    pub(crate) async fn replace_stream(&mut self, send_stream: Option<OwnedWriteHalf>) {
        *self.send_stream.lock().await = send_stream;
    }
}
