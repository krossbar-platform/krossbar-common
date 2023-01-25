use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use log::*;

use tokio::{net::UnixStream, sync::Mutex};

use karo_common_connection::connector::Connector;

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
        let mut connection = self.connector.connect().await?;
        trace!("RPC reconnected. Resubscribing");

        // XXX: Can we move this somewhere where we have [Writer]?
        self.call_registry
            .lock()
            .await
            .resend_subscriptions(&mut connection)
            .await?;

        Ok(connection)
    }
}
