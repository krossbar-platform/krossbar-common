use async_trait::async_trait;
use bson::Bson;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum MessageDirection {
    Outgoing,
    Incoming,
}

/// Structure to monitor message exchange for [Connection]
#[async_trait]
pub trait Monitor: Send + Sync {
    async fn message(&mut self, message: &Bson, direction: MessageDirection);
}
