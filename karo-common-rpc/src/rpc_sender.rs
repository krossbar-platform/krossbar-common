use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use anyhow::{Context, Ok, Result};
use bson::Bson;
use log::*;
use tokio::{
    net::UnixStream,
    sync::{mpsc, Mutex},
};

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
    pub async fn call(&mut self, body: Bson) -> Result<OneReceiver<UserMessageHandle>> {
        let receiver = self.call_subscribe(body, false).await?;

        Ok(OneReceiver::new(receiver))
    }

    /// Subscribe
    pub async fn subscribe(&mut self, body: Bson) -> Result<Receiver<UserMessageHandle>> {
        let receiver = self.call_subscribe(body, true).await?;

        Ok(Receiver::new(receiver))
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

// TODO: Move to own module?
/// Struct which drop on receiving a single message
pub struct OneReceiver<T> {
    receiver: mpsc::Receiver<T>,
}

impl<T> OneReceiver<T> {
    pub(crate) fn new(receiver: mpsc::Receiver<T>) -> Self {
        Self { receiver }
    }

    pub async fn recv(mut self) -> Option<T> {
        self.receiver.recv().await
    }
}

pub struct Receiver<T> {
    receiver: mpsc::Receiver<T>,
}

impl<T> Receiver<T> {
    pub(crate) fn new(receiver: mpsc::Receiver<T>) -> Self {
        Self { receiver }
    }

    pub async fn recv(&mut self) -> Option<T> {
        self.receiver.recv().await
    }
}
