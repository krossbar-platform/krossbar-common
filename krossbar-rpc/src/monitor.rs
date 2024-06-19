use std::sync::atomic::{AtomicBool, Ordering};

use futures::lock::Mutex;
use log::{debug, error};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tokio::net::UnixStream;

use crate::{message::RpcMessage, rpc::Rpc};

static MONITOR_ACTIVE: AtomicBool = AtomicBool::new(false);
static MONITOR_HANDLE: Lazy<Mutex<Option<Rpc>>> = Lazy::new(|| Mutex::new(None));

pub const MESSAGE_METHOD: &str = "message";

#[derive(Serialize, Deserialize, Debug)]
pub enum Direction {
    Incoming,
    Outgoing,
}

/// Monitor message
#[derive(Serialize, Deserialize, Debug)]
pub struct MonitorMessage {
    /// Peer name
    pub peer_name: String,
    /// Message direction
    pub direction: Direction,
    /// Message body
    pub message: RpcMessage,
}

pub struct Monitor;

impl Monitor {
    pub async fn set(stream: UnixStream) {
        debug!("Monitor connected");

        *MONITOR_HANDLE.lock().await = Some(Rpc::new(stream, "monitor"));
        MONITOR_ACTIVE.store(true, Ordering::Relaxed);
    }

    pub(crate) async fn send(message: &RpcMessage, direction: Direction, peer_name: &str) {
        if !MONITOR_ACTIVE.load(Ordering::Relaxed) {
            return;
        }

        let monitor_message = MonitorMessage {
            peer_name: peer_name.to_owned(),
            direction,
            message: message.clone(),
        };

        if let Some(rpc) = MONITOR_HANDLE.lock().await.as_ref() {
            if let Err(_) = rpc.send_message(MESSAGE_METHOD, &monitor_message).await {
                MONITOR_ACTIVE.store(false, Ordering::Relaxed);

                debug!("Monitor disconnected");
            }
        } else {
            error!("No monitor handle when monitor indicator is active. Please submit a bug")
        }
    }
}
