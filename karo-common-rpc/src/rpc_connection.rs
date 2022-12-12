use anyhow::{Context, Result};
use bson::{self, Bson};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, sync::mpsc::Receiver};

use karo_common_connection::{connection::Connection, connector::Connector};

use crate::{
    message::{Message, MessageType},
    user_message::UserMessageHandle, call_registry::CallRegistry,
};

pub struct RpcConnection<S: AsyncReadExt + AsyncWriteExt> {
    seq_no_counter: u64,
    connection: Connection<S>,
    call_registry: CallRegistry,
}

impl<S: AsyncReadExt + AsyncWriteExt + Unpin> RpcConnection<S> {
    /// Contructor. Uses [Connector] to connect to the peer
    pub async fn new(connector: Box<dyn Connector<S>>) -> Result<Self> {
        Ok(Self {
            seq_no_counter: 0,
            connection: Connection::new(connector).await?,
            call_registry: CallRegistry::new(),
        })
    }

    /// Read incoming messages
    pub async fn read(&mut self) -> Result<UserMessageHandle> {
        // The function loops if received a response. In this case we send the reponse
        // to a user using a future from the call registry and read nex message.
        loop {
            let incoming_bson = self.connection.read_bson().await?;

            let incoming_message = bson::from_bson::<Message>(incoming_bson)
                .context("Failed to deserialize incoming message")?;

            match incoming_message.message_type {
                MessageType::Call => {
                    return Ok(UserMessageHandle::new_call(
                        incoming_message,
                        self.connection.writer(),
                    ))
                }
                MessageType::Message => return Ok(UserMessageHandle::new(incoming_message)),
                MessageType::Response => self.call_registry.resolve(UserMessageHandle::new(incoming_message)).await
            }
        }
    }

    /// Send a one-way message
    pub async fn send(&mut self, body: Bson) -> Result<()> {
        let message = Message::new(self.seq_no(), body);
        let message = bson::to_bson(&message).context("Failed to serialise a message")?;

        self.connection.writer().write_bson(&message).await
    }

    /// Send a call
    pub async fn call(&mut self, body: Bson, subscription: bool) -> Result<Receiver<UserMessageHandle>> {
        let message = Message::new_call(self.seq_no(), body);
        let bson = bson::to_bson(&message).context("Failed to serialise a call")?;

        let rx = self.call_registry.register_call(message, subscription).context("Failed to register a call")?;

        self.connection.writer().write_bson(&bson).await.context("Failed to write outgoing message")?;
        Ok(rx)
    }

    /// Reset all existing calls on reconnect
    pub fn reset(&mut self) {
        self.call_registry.clear()
    }

    fn seq_no(&mut self) -> u64 {
        self.seq_no_counter += 1;
        self.seq_no_counter
    }
}
