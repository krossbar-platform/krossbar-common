use std::os::unix::{
    net::UnixStream as OsStream,
    prelude::{FromRawFd, IntoRawFd},
};

use anyhow::{Context, Result};
use bson::{self, Bson};
use bytes::Bytes;
use tokio::{
    io::AsyncWriteExt,
    net::UnixStream,
    sync::mpsc::{self, Receiver},
};
use tokio_send_fd::SendFd;

use crate::{
    connector::Connector,
    monitor::{MessageDirection, Monitor},
    socket_reader::read_bson_from_socket,
    writer::Writer,
};

/// Generic connection handle.
/// Reconnects if connection closed.
/// Can be used to get a Writer to simultaniously send data
pub struct Connection {
    /// Connector to initiate connection and reconnect if disconnected
    connector: Box<dyn Connector>,
    /// Read half of a socket connected to a peer
    stream: UnixStream,
    /// Write channel receiver to receive outgoing messages
    outgoing_rx: Receiver<(Bson, Option<UnixStream>)>,
    /// To return to users to write outgoing messages
    writer: Writer,
    /// Optional message exchange monitor
    monitor: Option<Box<dyn Monitor>>,
}

impl Connection {
    /// Contructor. Uses [Connector] to connect to the peer
    pub async fn new(connector: Box<dyn Connector>) -> Result<Self> {
        let read_stream = connector.connect().await?;
        let (outgoing_tx, outgoing_rx) = mpsc::channel(32);

        Ok(Self {
            connector,
            stream: read_stream,
            outgoing_rx,
            writer: Writer::new(outgoing_tx),
            monitor: None,
        })
    }

    /// Read Bson from the socket
    pub async fn read_bson(&mut self) -> Result<Bson> {
        let received_data = self
            .read()
            .await
            .context("Failed to read incoming message")?;

        let bson = bson::from_slice(&received_data).context("Failed to deserialize incoming Bson");

        if let (Some(monitor), Ok(body)) = (&mut self.monitor, &bson) {
            monitor.message(body, MessageDirection::Incoming).await;
        }

        bson
    }

    /// Read fd from the socket
    /// This should be used only if you're sure that sender sent the fd rigth after the message
    /// you've just read
    pub async fn read_fd(&mut self) -> Result<UnixStream> {
        let fd = self.stream.recv_fd().await?;

        let os_stream = unsafe { OsStream::from_raw_fd(fd) };
        UnixStream::from_std(os_stream).context("Failed to create UnixStream from fd")
    }

    pub fn set_monitor(&mut self, monitor: Box<dyn Monitor>) {
        self.monitor = Some(monitor);
    }

    /// Read raw Bson data from the socket
    async fn read(&mut self) -> Result<Bytes> {
        loop {
            tokio::select! {
                message = read_bson_from_socket(&mut self.stream, false) => {
                    if message.is_ok() {
                        return message
                    } else {
                        self.do_reconnect().await?;
                    }
                },
                Some((outgoing_message, stream)) = self.outgoing_rx.recv() => {
                    let fd = stream.map(|stream| stream.into_std().unwrap().into_raw_fd());

                    if let Some(monitor) = &mut self.monitor {
                        monitor.message(&outgoing_message, MessageDirection::Outgoing).await;
                    }

                    loop {
                        let bytes = Bytes::from(
                            bson::to_raw_document_buf(&outgoing_message)
                                .map(|data| data.into_bytes())
                                .context("Failed to serialize incoming Bson")?,
                        );

                        if self.stream.write(&bytes).await.is_err() {
                            self.do_reconnect().await?;
                            continue;
                        }

                        if let Some(fd) = fd {
                            if self.stream.send_fd(fd).await.is_err() {
                                self.do_reconnect().await?;
                                continue;
                            }
                        }

                        break;
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
                return Err(err);
            }
        };

        self.stream = stream;

        // Give connector an opportunity to send post-connection messages
        self.connector.on_connected(&mut self.writer).await
    }

    /// Get a new writer
    pub fn writer(&self) -> Writer {
        self.writer.clone()
    }
}
