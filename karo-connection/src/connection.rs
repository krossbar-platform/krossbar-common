use anyhow::{Context, Result};
use bson::{self, Bson};
use bytes::Bytes;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::mpsc::{self, Receiver},
};

use crate::{connector::Connector, socket_reader::read_bson_from_socket, writer::Writer};

/// Generic connection handle.
/// Reconnects if connection closed.
/// Can be used to get a Writer to simultaniously send data
pub struct Connection<S: AsyncReadExt + AsyncWriteExt> {
    /// Connector to initiate connection and reconnect if disconnected
    connector: Box<dyn Connector<S>>,
    /// Socket connected to a peer
    stream: S,
    /// To receive data from a writer
    write_rx: Receiver<Bytes>,
    /// To return to users to write outgoing messages
    writer: Writer,
}

impl<S: AsyncReadExt + AsyncWriteExt + Unpin> Connection<S> {
    /// Contructor. Uses [Connector] to connect to the peer
    pub async fn new(connector: Box<dyn Connector<S>>) -> Result<Self> {
        let (sender, receiver) = mpsc::channel(32);
        let stream = connector.connect().await?;

        Ok(Self {
            connector,
            stream,
            write_rx: receiver,
            writer: Writer::new(sender),
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

    /// Read raw Bson data from the socket
    async fn read(&mut self) -> Result<Bytes> {
        loop {
            tokio::select! {
                message = read_bson_from_socket(&mut self.stream, false) => {
                    if message.is_ok() {
                        return message
                    } else {
                        self.stream = self.connector.connect().await?;
                    }
                }
                data = self.write_rx.recv() => {
                    if data.is_some() &&  self.stream.write_all(&data.unwrap()).await.is_err() {
                        self.stream = self.connector.connect().await?;
                    }
                }
            }
        }
    }

    /// Get a new writer
    pub fn writer(&self) -> Writer {
        self.writer.clone()
    }
}
