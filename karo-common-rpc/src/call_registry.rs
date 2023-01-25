use std::collections::HashMap;

use anyhow::{Context, Result};
use bson;
use log::*;
use tokio::{
    io::AsyncWriteExt,
    sync::mpsc::{self, Receiver, Sender},
};

use crate::{message::Message, user_message::UserMessageHandle};

/// Call handle we save in a map
struct Call {
    /// If we're gonna have multiple replies to the call (for instance in case of subscriptions)
    subscription: bool,
    /// Message to resend on resubscription request
    message: Message,
    /// Use callback TX
    callback: Sender<UserMessageHandle>,
}

/// Struct to account user calls and send incoming response
/// to a proper caller.
pub struct CallRegistry {
    /// Calls map
    calls: HashMap<u64, Call>,
}

impl CallRegistry {
    pub fn new() -> Self {
        Self {
            calls: HashMap::new(),
        }
    }

    /// Register a call
    /// *persist* If the call will have multiple answers
    ///
    /// *subscription* if we want to keep the call end resubscribe on reconnection
    /// *read_fd* if client await a file descriptor right after the response
    ///
    /// *Returns* Receiver for a caller
    pub fn register_call(
        &mut self,
        message: Message,
        subscription: bool,
    ) -> Result<Receiver<UserMessageHandle>> {
        let (tx, rx) = mpsc::channel(16);

        trace!("Registering a call: {:?}", message);

        if self.calls.contains_key(&message.seq_no) {
            panic!("Multiple calls with the same ID: {}", message.seq_no);
        }

        self.calls.insert(
            message.seq_no,
            Call {
                subscription,
                message,
                callback: tx,
            },
        );

        Ok(rx)
    }

    /// Resolve a client call with a given *message*
    pub async fn resolve(&mut self, handle: UserMessageHandle) {
        let seq_no = handle.id();

        match self.calls.get(&seq_no) {
            Some(call) => {
                trace!("Found a call with id: {}", seq_no);

                if let Err(mess) = call.callback.send(handle).await {
                    error!(
                        "Failed to write response message {} into a channel: {:?}",
                        seq_no,
                        mess.to_string()
                    );
                    return;
                }

                if !call.subscription {
                    trace!("Removing resolved call: {}", seq_no);

                    self.calls.remove(&seq_no);
                }
            }
            _ => {
                warn!("Unknown client response: {:?}", handle.body());
            }
        }
    }

    pub async fn resend_subscriptions<S: AsyncWriteExt + Unpin>(
        &self,
        socket: &mut S,
    ) -> Result<()> {
        for message in self.calls.values() {
            if !message.subscription {
                continue;
            }

            let bytes = bson::to_raw_document_buf(&message.message)
                .map(|data| data.into_bytes())
                .context("Failed to serialize incoming Bson")?;

            socket
                .write_all(&bytes)
                .await
                .context("Failed to write outgoing message into the socket")?;
        }

        Ok(())
    }

    /// Reset all calls on reconnect
    pub fn clear(&mut self) {
        self.calls.clear();
    }
}
