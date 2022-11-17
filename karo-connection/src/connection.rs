use anyhow::Result;
use bytes::Bytes;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::mpsc::{self, Receiver},
};

use crate::{connector::Connector, socket_reader::read_bson_from_socket, writer::Writer};

pub struct Connection<S: AsyncReadExt + AsyncWriteExt> {
    /// Connector to initiate connection and reconnect if disconnected
    connector: Box<dyn Connector<S>>,
    /// UDS connect to the counterparty
    stream: S,
    /// To receive data from a writer
    write_rx: Receiver<Bytes>,
    /// To return to users
    writer: Writer,
}

impl<S: AsyncReadExt + AsyncWriteExt + Unpin> Connection<S> {
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

    pub async fn read(&mut self) -> Result<Bytes> {
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

    pub fn writer(&self) -> Writer {
        self.writer.clone()
    }
}
