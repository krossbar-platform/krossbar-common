use std::collections::HashMap;

use bson::Bson;
use futures::{
    channel::{
        mpsc::{channel, Receiver, Sender},
        oneshot::{channel as one_channel, Receiver as OneReceiver, Sender as OneSender},
    },
    SinkExt,
};
use log::{debug, info, warn};
use tokio::net::UnixStream;

const BAD_RESPONSE_QUEUE_SIZE: usize = 100;

pub struct CallsRegistry {
    id_counter: i64,
    calls: HashMap<i64, OneSender<crate::Result<Bson>>>,
    fd_calls: HashMap<i64, OneSender<crate::Result<(Bson, UnixStream)>>>,
    subscriptions: HashMap<i64, Sender<crate::Result<Bson>>>,
}

impl CallsRegistry {
    pub fn new() -> Self {
        Self {
            id_counter: 0,
            calls: HashMap::new(),
            fd_calls: HashMap::new(),
            subscriptions: HashMap::new(),
        }
    }

    pub fn add_call(&mut self) -> (i64, OneReceiver<crate::Result<Bson>>) {
        let (sender, receiver) = one_channel();
        let id = self.next_id();

        self.calls.insert(id, sender);
        (id, receiver)
    }

    pub fn add_fd_call(&mut self) -> (i64, OneReceiver<crate::Result<(Bson, UnixStream)>>) {
        let (sender, receiver) = one_channel();
        let id = self.next_id();

        self.fd_calls.insert(id, sender);
        (id, receiver)
    }

    pub fn add_subscription(&mut self) -> (i64, Receiver<crate::Result<Bson>>) {
        let (sender, receiver) = channel(BAD_RESPONSE_QUEUE_SIZE);
        let id = self.next_id();

        self.subscriptions.insert(id, sender);
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
                        if let Err(_) = channel.send(Ok((doc, stream))) {
                            warn!("User wasn't waiting for an fd call response")
                        }
                    } else {
                        warn!("Ok response for a stream request w/o the stream itself");
                        if let Err(_) = channel.send(Err(crate::Error::ProtocolError)) {
                            warn!("User wasn't waiting for an fd call response")
                        }
                    }
                }
                Err(e) => {
                    if let Err(_) = channel.send(Err(e)) {
                        warn!("User wasn't waiting for an fd call response")
                    }
                }
            }
        } else {
            warn!("Received unexpected response. Call registry doesn't have matching request")
        }
    }

    pub async fn resolve(&mut self, message_id: i64, response: crate::Result<Bson>) {
        debug!("Incoming response for {message_id}: {response:?}");

        if let Some(channel) = self.calls.remove(&message_id) {
            if let Err(_) = channel.send(response) {
                warn!("User wasn't waiting for a call response")
            }
        } else if let Some(channel) = self.subscriptions.get_mut(&message_id) {
            if let Err(_) = channel.send(response).await {
                warn!("User wasn't waiting for a subscription response")
            }
        } else {
            warn!("Unexpected peer response: {:?}", response)
        }
    }

    pub fn clear(&mut self) {
        info!("Clearing calls queue");
        self.calls.clear()
    }

    fn next_id(&mut self) -> i64 {
        self.id_counter += 1;
        self.id_counter
    }
}
