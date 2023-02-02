use std::sync::Arc;

use async_trait::async_trait;
use bson::Bson;
use karo_common_connection::monitor::{MessageDirection, Monitor};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AccMonitor {
    messages: Arc<Mutex<Vec<(Bson, MessageDirection)>>>,
}

impl AccMonitor {
    pub fn messages(&self) -> &Arc<Mutex<Vec<(Bson, MessageDirection)>>> {
        &self.messages
    }

    pub async fn print(&self) -> String {
        let mut result = String::new();

        for (bson, direction) in self.messages.lock().await.iter() {
            let direction_str = match direction {
                MessageDirection::Incoming => "incoming".to_owned(),
                MessageDirection::Outgoing => "outgoing".to_owned(),
            };

            result.push_str(&format!("{}: {}\n", direction_str, bson));
        }

        result
    }
}

#[async_trait]
impl Monitor for AccMonitor {
    async fn message(&mut self, message: &Bson, direction: MessageDirection) {
        self.messages
            .lock()
            .await
            .push((message.clone(), direction));
    }
}

impl Default for AccMonitor {
    fn default() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
        }
    }
}
