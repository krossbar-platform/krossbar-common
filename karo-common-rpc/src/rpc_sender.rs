use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use anyhow::{anyhow, Context, Ok, Result};
use bson::Bson;
use log::*;
use serde::Serialize;
use tokio::{
    net::UnixStream,
    sync::{mpsc, Mutex},
};
use tokio_stream::wrappers::ReceiverStream;

use karo_common_connection::writer::Writer;

use crate::{call_registry::CallRegistry, message::Message, user_message::UserMessageHandle};

#[derive(Clone)]
pub struct RpcSender {
    seq_no_counter: Arc<AtomicU64>,
    socket_writer: Writer,
    call_registry: Arc<Mutex<CallRegistry>>,
}

impl RpcSender {
    pub fn new(socket_writer: Writer, call_registry: Arc<Mutex<CallRegistry>>) -> Self {
        Self {
            seq_no_counter: Arc::new(0u64.into()),
            socket_writer,
            call_registry,
        }
    }

    /// Send a one-way message
    pub async fn send(&mut self, body: Bson) -> Result<()> {
        let message = Message::new(self.seq_no(), body, false);
        trace!("Sending new message: {:?}", message);

        let message = bson::to_bson(&message).context("Failed to serialise a message")?;

        self.socket_writer.write_bson(&message).await
    }

    /// Send a one-way message
    pub async fn send_fd(&mut self, body: Bson, stream: UnixStream) -> Result<()> {
        let message = Message::new(self.seq_no(), body, true);
        trace!("Sending new message: {:?}", message);

        let message = bson::to_bson(&message).context("Failed to serialise a message")?;

        self.socket_writer.write_bson_fd(&message, stream).await
    }

    /// Send a call
    pub async fn call<M: Serialize>(&mut self, message: &M) -> Result<UserMessageHandle> {
        let body = bson::to_bson(message).unwrap();
        let mut receiver = self.call_subscribe(body, false).await?;

        receiver.recv().await.ok_or(anyhow!("Connection closed"))
    }

    /// Subscribe
    pub async fn subscribe<M: Serialize>(
        &mut self,
        message: &M,
    ) -> Result<ReceiverStream<UserMessageHandle>> {
        let body = bson::to_bson(message).unwrap();
        let receiver = self.call_subscribe(body, true).await?;

        Ok(ReceiverStream::new(receiver))
    }

    async fn call_subscribe(
        &mut self,
        body: Bson,
        subscription: bool,
    ) -> Result<mpsc::Receiver<UserMessageHandle>> {
        let message = Message::new_call(self.seq_no(), body, false);
        trace!("Sending new call: {:?}", message);

        let bson = bson::to_bson(&message).context("Failed to serialise a call")?;

        let rx = self
            .call_registry
            .lock()
            .await
            .register_call(message, subscription)
            .context("Failed to register a call")?;

        self.socket_writer
            .write_bson(&bson)
            .await
            .context("Failed to write outgoing message")?;
        Ok(rx)
    }

    fn seq_no(&mut self) -> u64 {
        self.seq_no_counter.fetch_add(1, Ordering::Release)
    }
}
