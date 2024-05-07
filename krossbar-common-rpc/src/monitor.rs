use std::sync::atomic::{AtomicBool, Ordering};

use futures::{lock::Mutex, Stream};
use log::{debug, error};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tokio::net::UnixStream;

use crate::{message::RpcMessage, message_stream::AsyncWriteMessage};

static MONITOR_ACTIVE: AtomicBool = AtomicBool::new(false);
static MONITOR_HANDLE: Lazy<Mutex<Option<UnixStream>>> = Lazy::new(|| Mutex::new(None));

#[derive(Serialize, Deserialize, Debug)]
pub enum Direction {
    Incoming,
    Ougoing,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MonitorMessage {
    pub direction: Direction,
    pub message: RpcMessage,
}

pub struct Monitor;

impl Monitor {
    pub async fn set(stream: UnixStream) {
        debug!("Monitor connected");

        *MONITOR_HANDLE.lock().await = Some(stream);
        MONITOR_ACTIVE.store(true, Ordering::Relaxed);
    }

    #[cfg(feature = "impl-monitor")]
    pub fn make_receiver(mut stream: UnixStream) -> impl Stream<Item = MonitorMessage> {
        use std::pin::pin;

        use futures::{stream, Future};

        use crate::message_stream::AsyncReadMessage;

        stream::poll_fn(move |ctx| {
            pin!(stream.read_message()).poll(ctx).map(|val| match val {
                Ok(message) => Some(message),
                Err(_) => None,
            })
        })
    }

    pub(crate) async fn send(message: &RpcMessage, direction: Direction) {
        if !MONITOR_ACTIVE.load(Ordering::Relaxed) {
            return;
        }

        let monitor_message = MonitorMessage {
            direction,
            message: message.clone(),
        };

        let mut lock = MONITOR_HANDLE.lock().await;
        if let Some(stream) = lock.as_mut() {
            if let Err(_) = stream.write_message(&monitor_message).await {
                MONITOR_ACTIVE.store(false, Ordering::Relaxed);

                debug!("Monitor disconnected");
            }
        } else {
            error!("No monitor handle when monitor indicator is active. Please submit a bug")
        }
    }
}
