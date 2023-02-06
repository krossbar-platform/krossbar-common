use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use log::*;

use tokio::{net::UnixStream, sync::Mutex};

use karo_common_connection::{connector::Connector, writer::Writer};

use crate::call_registry::CallRegistry;

/// Connector which wraps another [Connector] and performs resubscription after reconnection
pub struct RpcConnector {
    connector: Box<dyn Connector>,
    call_registry: Arc<Mutex<CallRegistry>>,
}

impl RpcConnector {
    pub fn new(connector: Box<dyn Connector>, call_registry: Arc<Mutex<CallRegistry>>) -> Self {
        Self {
            connector,
            call_registry,
        }
    }
}

#[async_trait]
impl Connector for RpcConnector {
    async fn connect(&self) -> Result<UnixStream> {
        self.connector.connect().await
    }

    async fn on_connected(&self, writer: &mut Writer) -> Result<()> {
        trace!("RPC reconnected. Resubscribing");

        self.call_registry
            .lock()
            .await
            .resend_subscriptions(writer)
            .await
    }
}
