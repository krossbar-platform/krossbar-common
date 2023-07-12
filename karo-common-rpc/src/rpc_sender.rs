use std::sync::Arc;

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

use crate::{
    call_registry::CallRegistry, message::Message, sequential_id_provider::SequentialIdProvider,
    user_message::UserMessageHandle,
};

/// Rpc sender, which user uses to send RPC messages
#[derive(Clone)]
pub struct RpcSender {
    /// Source writer in which messages will be written
    socket_writer: Writer,
    /// Call registry which accounts for RPC calls and resubscribes on
    /// reconnections
    call_registry: Arc<Mutex<CallRegistry>>,
    /// Sequential ID provider to receive monotonic mesage IDs
    seq_id_provider: SequentialIdProvider,
}

impl RpcSender {
    pub fn new(
        socket_writer: Writer,
        call_registry: Arc<Mutex<CallRegistry>>,
        seq_id_provider: SequentialIdProvider,
    ) -> Self {
        Self {
            socket_writer,
            call_registry,
            seq_id_provider,
        }
    }

    /// Send a one-way message
    pub async fn send(&mut self, body: Bson) -> Result<()> {
        let message = Message::new(self.seq_id_provider.next_id(), body, false);
        trace!("Sending new message: {:?}", message);

        let message = bson::to_bson(&message).context("Failed to serialise a message")?;

        self.socket_writer.write_bson(&message).await
    }

    /// Send a one-way message
    pub async fn send_fd(&mut self, body: Bson, stream: UnixStream) -> Result<()> {
        let message = Message::new(self.seq_id_provider.next_id(), body, true);
        trace!("Sending new message: {:?}", message);

        let message = bson::to_bson(&message).context("Failed to serialise a message")?;

        self.socket_writer.write_bson_fd(&message, stream).await
    }

    /// Send a call
    pub async fn call<M: Serialize>(&mut self, message: &M) -> Result<UserMessageHandle> {
        let body = bson::to_bson(message).unwrap();
        let mut receiver = self.call_subscribe(body, false).await?;

        receiver
            .recv()
            .await
            .ok_or_else(|| anyhow!("Connection closed"))
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
        let seq_no = self.seq_id_provider.next_id();

        let message = Message::new_call(seq_no, body, false);
        trace!("Sending new call: {:?}", message);

        let bson = bson::to_bson(&message).context("Failed to serialise a call")?;

        let rx = self
            .call_registry
            .lock()
            .await
            .register_call(seq_no, bson.clone(), subscription)
            .context("Failed to register a call")?;

        self.socket_writer
            .write_bson(&bson)
            .await
            .context("Failed to write outgoing message")?;
        Ok(rx)
    }
}
