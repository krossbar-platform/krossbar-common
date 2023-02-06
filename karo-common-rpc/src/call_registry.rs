use std::collections::HashMap;

use anyhow::Result;
use bson::{self, Bson};
use karo_common_connection::writer::Writer;
use log::*;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::user_message::UserMessageHandle;

/// Call handle we save in a map
struct Call {
    /// If we're gonna have multiple replies to the call (for instance in case of subscriptions)
    subscription: bool,
    /// Message to resend on resubscription request
    message: Bson,
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
        seq_no: u64,
        message: Bson,
        subscription: bool,
    ) -> Result<Receiver<UserMessageHandle>> {
        let (tx, rx) = mpsc::channel(16);

        trace!("Registering a call: {:?}", message);

        if self.calls.contains_key(&seq_no) {
            panic!("Multiple calls with the same ID: {}", seq_no);
        }

        self.calls.insert(
            seq_no,
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
                warn!("Unknown client response: {:?}", handle.body::<Bson>());
            }
        }
    }

    pub async fn resend_subscriptions(&self, writer: &mut Writer) -> Result<()> {
        for call in self.calls.values() {
            if !call.subscription {
                continue;
            }

            writer.write_bson(&call.message).await?
        }

        Ok(())
    }

    /// Reset all calls on reconnect
    pub fn clear(&mut self) {
        self.calls.clear();
    }
}

impl Default for CallRegistry {
    fn default() -> Self {
        Self::new()
    }
}
