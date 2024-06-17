use std::collections::HashMap;

use bson::Bson;
use futures::{
    channel::{
        mpsc::{channel, Receiver, Sender},
        oneshot::{channel as one_channel, Receiver as OneReceiver, Sender as OneSender},
    },
    SinkExt,
};
#[cfg(not(feature = "log-to-stdout"))]
use log::{debug, info, trace, warn};
use tokio::net::UnixStream;

use crate::message::{self};
#[cfg(feature = "log-to-stdout")]
use crate::{debug, info, trace, warn};

const BAD_RESPONSE_QUEUE_SIZE: usize = 100;

/// A registry of calls, which can be added, resulting in a call id, and resolved after
/// the client has responded
pub struct CallsRegistry {
    id_counter: i64,
    /// Active calls
    calls: HashMap<i64, OneSender<crate::Result<Bson>>>,
    /// Active FD calls
    fd_calls: HashMap<i64, OneSender<crate::Result<(Bson, UnixStream)>>>,
    /// Current subscriptions
    subscriptions: HashMap<i64, Sender<crate::Result<Bson>>>,
    /// Persistent calls to send on reconnect
    active_subscriptions: HashMap<i64, message::RpcData>,
}

impl CallsRegistry {
    pub fn new() -> Self {
        Self {
            id_counter: 0,
            calls: HashMap::new(),
            fd_calls: HashMap::new(),
            subscriptions: HashMap::new(),
            active_subscriptions: HashMap::new(),
        }
    }

    pub fn add_persistent_call(&mut self, id: i64, data: message::RpcData) {
        assert!(!self.active_subscriptions.contains_key(&id));

        self.active_subscriptions.insert(id, data);
    }

    pub fn add_call(&mut self) -> (i64, OneReceiver<crate::Result<Bson>>) {
        let (sender, receiver) = one_channel();
        let id = self.next_id();

        self.calls.insert(id, sender);

        trace!("Add new call");

        (id, receiver)
    }

    pub fn add_fd_call(&mut self) -> (i64, OneReceiver<crate::Result<(Bson, UnixStream)>>) {
        let (sender, receiver) = one_channel();
        let id = self.next_id();

        self.fd_calls.insert(id, sender);

        trace!("Add new FD call");

        (id, receiver)
    }

    pub fn add_subscription(&mut self) -> (i64, Receiver<crate::Result<Bson>>) {
        let (sender, receiver) = channel(BAD_RESPONSE_QUEUE_SIZE);
        let id = self.next_id();

        self.subscriptions.insert(id, sender);

        trace!("Add new subscription");

        (id, receiver)
    }

    pub fn resolve_with_fd(
        &mut self,
        message_id: i64,
        response: crate::Result<Bson>,
        maybe_fd: Option<UnixStream>,
    ) {
        debug!("Incoming fd response for {message_id}: {response:?}");

        if let Some(channel) = self.fd_calls.remove(&message_id) {
            match response {
                Ok(doc) => {
                    if let Some(stream) = maybe_fd {
                        if channel.send(Ok((doc, stream))).is_err() {
                            warn!("User wasn't waiting for an fd call response")
                        } else {
                            debug!("Succesfully resolved FD response for a message {message_id}")
                        }
                    } else {
                        warn!("Ok response for a stream request w/o the stream itself");
                        if channel
                            .send(Err(crate::Error::InternalError(
                                "Hub returned Ok response without a stream handle".into(),
                            )))
                            .is_err()
                        {
                            warn!("User wasn't waiting for an fd call response")
                        }
                    }
                }
                Err(e) => {
                    if channel.send(Err(e)).is_err() {
                        warn!("User wasn't waiting for an fd call response")
                    }
                }
            }
        } else {
            warn!("Received unexpected response. Call registry doesn't have matching request")
        }
    }

    pub async fn resolve(&mut self, message_id: i64, response: crate::Result<Bson>) {
        // Try to resolve an active call
        if let Some(channel) = self.calls.remove(&message_id) {
            if channel.send(response).is_err() {
                warn!("User dropped call handle. Failed to send a response")
            } else {
                debug!("Succesfully resolved {message_id} call")
            }
        } else if let Some(channel) = self.subscriptions.get_mut(&message_id) {
            // Subscription is no longer active
            if !self.active_subscriptions.contains_key(&message_id) {
                debug!("Inactive subscription response")
            // Try to resolve an active subscription
            } else if channel.send(response).await.is_err() {
                warn!("User dropped subscriptions handle. Failed to send a response");

                // Remove subscription as no one is waiting for it anymore
                self.active_subscriptions.remove(&message_id);
            // No such call
            } else {
                debug!("Succesfully resolved {message_id} subscription")
            }
        // If we've received an error for FD call, we may not know it's actually FD response
        // Let's check it manually
        } else if self.fd_calls.contains_key(&message_id) {
            self.resolve_with_fd(message_id, response, None)
        } else {
            warn!("Unexpected peer response: {:?}", response)
        }
    }

    pub fn clear_pending_calls(&mut self) {
        info!("Clearing calls queue");
        self.calls.clear()
    }

    pub fn active_subscriptions(&self) -> impl Iterator<Item = (&i64, &message::RpcData)> {
        self.active_subscriptions.iter()
    }

    fn next_id(&mut self) -> i64 {
        self.id_counter += 1;
        self.id_counter
    }
}
