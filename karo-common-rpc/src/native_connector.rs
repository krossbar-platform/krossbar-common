use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use log::*;

use tokio::{net::UnixStream, sync::Mutex};

use karo_common_connection::{connector::Connector, writer::Writer};

use crate::{
    call_registry::CallRegistry, rpc_connector::RpcConnector, rpc_sender::RpcSender,
    sequential_id_provider::SequentialIdProvider,
};

/// Connector which wraps source [RpcConnector] to pass it to a native connection.
/// Resubscribes after reconnection.
/// Also used to transform [karo_common_connection::writer::Writer] into [RpcSender] on reconnect.
pub struct NativeConnector {
    /// User-provided struct, which knows how to connect a socket
    connector: Box<dyn RpcConnector>,
    /// Call registry which accounts for RPC calls and resubscribes on
    /// reconnections
    call_registry: Arc<Mutex<CallRegistry>>,
    /// Sequential ID provider to receive monotonic mesage IDs
    seq_id_provider: SequentialIdProvider,
}

impl NativeConnector {
    pub fn new(
        connector: Box<dyn RpcConnector>,
        call_registry: Arc<Mutex<CallRegistry>>,
        seq_id_provider: SequentialIdProvider,
    ) -> Self {
        Self {
            connector,
            call_registry,
            seq_id_provider,
        }
    }
}

#[async_trait]
impl Connector for NativeConnector {
    async fn connect(&self) -> Result<UnixStream> {
        self.connector.connect().await
    }

    async fn on_connected(&self, writer: &mut Writer) -> Result<()> {
        trace!("RPC reconnected. Resubscribing");

        self.call_registry
            .lock()
            .await
            .resend_subscriptions(writer)
            .await?;

        // First we make [RpcSender] to give it to a user
        let mut rpc_sender = RpcSender::new(
            writer.clone(),
            self.call_registry.clone(),
            self.seq_id_provider.clone(),
        );

        trace!("Calling users callback");
        self.connector.on_connected(&mut rpc_sender).await
    }
}
