use std::sync::Arc;

use anyhow::{Context, Result};
use bson;
use log::*;
use tokio::sync::Mutex;

use karo_common_connection::{connection::Connection, monitor::Monitor};

use crate::{
    call_registry::CallRegistry,
    message::{Message, MessageType},
    native_connector::NativeConnector,
    rpc_connector::RpcConnector,
    rpc_sender::RpcSender,
    sequential_id_provider::SequentialIdProvider,
    user_message::UserMessageHandle,
};

/// RPC connection handle.
/// Uses [CallRegistry] to account user calls.
/// Uses [Connector] wrapper to resubscribe if reconnected
pub struct RpcConnection {
    /// Socket connection to send/receive data
    connection: Connection,
    /// Common sender, which can be used to clone and return to a user
    sender: RpcSender,
    /// Call registry, which is used to record calls, resubscribe on reconnection and send user responses
    call_registry: Arc<Mutex<CallRegistry>>,
}

impl RpcConnection {
    /// Contructor. Uses [Connector] to connect to the peer
    pub async fn new(connector: Box<dyn RpcConnector>) -> Result<Self> {
        let seq_id_provider = SequentialIdProvider::new();
        let call_registry = Arc::new(Mutex::new(CallRegistry::new()));

        let rpc_connector =
            NativeConnector::new(connector, call_registry.clone(), seq_id_provider.clone());

        let connection = Connection::new(Box::new(rpc_connector)).await?;
        let sender = RpcSender::new(connection.writer(), call_registry.clone(), seq_id_provider);

        Ok(Self {
            connection,
            sender,
            call_registry,
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

            // Sender sent a file descriptor right after the message. Let's read it
            let optional_fd = if incoming_message.has_fd {
                debug!("Received a message with a file descriptor. Trying to read the descriptor");

                Some(self.connection.read_fd().await?)
            } else {
                None
            };

            match incoming_message.message_type {
                MessageType::Call => {
                    return Ok(UserMessageHandle::new_call(
                        incoming_message,
                        self.connection.writer(),
                    ))
                }
                MessageType::Message => {
                    return Ok(UserMessageHandle::new(incoming_message, optional_fd))
                }
                MessageType::Response => {
                    self.call_registry
                        .lock()
                        .await
                        .resolve(UserMessageHandle::new(incoming_message, optional_fd))
                        .await
                }
            }
        }
    }

    pub fn sender(&self) -> RpcSender {
        self.sender.clone()
    }

    /// Reset all existing calls on reconnect
    pub async fn reset(&mut self) {
        self.call_registry.lock().await.clear()
    }

    pub fn set_monitor(&mut self, monitor: Box<dyn Monitor>) {
        self.connection.set_monitor(monitor);
    }
}
