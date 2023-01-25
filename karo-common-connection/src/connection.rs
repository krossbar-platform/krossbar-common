use std::os::unix::{net::UnixStream as OsStream, prelude::FromRawFd};

use anyhow::{Context, Result};
use bson::{self, Bson};
use bytes::Bytes;
use tokio::net::{unix::OwnedReadHalf, UnixStream};
use tokio_send_fd::SendFd;

use crate::{connector::Connector, socket_reader::read_bson_from_socket, writer::Writer};

/// Generic connection handle.
/// Reconnects if connection closed.
/// Can be used to get a Writer to simultaniously send data
pub struct Connection {
    /// Connector to initiate connection and reconnect if disconnected
    connector: Box<dyn Connector>,
    /// Read half of a socket connected to a peer
    read_stream: OwnedReadHalf,
    /// To return to users to write outgoing messages
    writer: Writer,
    /// Reconnect if connection dropped
    reconnect: bool,
}

impl Connection {
    /// Contructor. Uses [Connector] to connect to the peer
    pub async fn new(connector: Box<dyn Connector>, reconnect: bool) -> Result<Self> {
        let (read_stream, write_stream) = connector.connect().await?.into_split();

        Ok(Self {
            connector,
            read_stream,
            writer: Writer::new(write_stream),
            reconnect,
        })
    }

    /// Read Bson from the socket
    pub async fn read_bson(&mut self) -> Result<Bson> {
        let received_data = self
            .read()
            .await
            .context("Failed to read incoming message")?;

        bson::from_slice(&received_data).context("Failed to deserialize incoming Bson")
    }

    /// Read fd from the socket
    /// This should be used only if you're sure that sender sent the fd rigth after the message
    /// you've just read
    pub async fn read_fd(&mut self) -> Result<UnixStream> {
        let fd = self.read_stream.as_ref().recv_fd().await?;

        let os_stream = unsafe { OsStream::from_raw_fd(fd) };
        UnixStream::from_std(os_stream).context("Failed to create UnixStream from fd")
    }

    /// Read raw Bson data from the socket
    async fn read(&mut self) -> Result<Bytes> {
        loop {
            tokio::select! {
                message = read_bson_from_socket(&mut self.read_stream, false) => {
                    if message.is_ok() {
                        return message
                    } else if self.reconnect {
                        self.do_reconnect().await?;
                    }
                }
            }
        }
    }

    async fn do_reconnect(&mut self) -> Result<()> {
        let stream = match self.connector.connect().await {
            Ok(stream) => stream,
            Err(err) => {
                // If failed to reconnect, we reset writers write half to shutdown the writers owners
                self.writer.replace_stream(None).await;
                return Err(err);
            }
        };
        let (read_stream, write_stream) = stream.into_split();

        self.read_stream = read_stream;
        self.writer.replace_stream(Some(write_stream)).await;
        Ok(())
    }

    /// Get a new writer
    pub fn writer(&self) -> Writer {
        self.writer.clone()
    }
}
