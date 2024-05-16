use std::{pin::Pin, sync::Arc};

use anyhow::bail;
use async_send_fd::AsyncSendTokioStream;
use futures::{lock::Mutex, stream::FusedStream, Future, FutureExt as _, StreamExt as _};
use log::{debug, trace, warn};
use serde::{de::DeserializeOwned, Serialize};
use tokio::net::{unix::OwnedWriteHalf, UnixStream};

use crate::message_stream::AsyncWriteMessage;

use super::{
    calls_registry::CallsRegistry,
    message::{self, RpcMessage},
};

type CallResultType<T> = crate::Result<Pin<Box<dyn Future<Output = crate::Result<T>> + Send>>>;
type SubResultType<T> = crate::Result<Pin<Box<dyn FusedStream<Item = crate::Result<T>> + Send>>>;

/// A writer to make RPC calls, or subscribe to the client
#[derive(Clone)]
pub struct RpcWriter {
    /// Writer part of the socket
    socket: Arc<Mutex<OwnedWriteHalf>>,
    /// Call registry to add outgoing calls into for later resolve
    registry: Arc<Mutex<CallsRegistry>>,
}

impl RpcWriter {
    /// Make a new writer from a reading half of the stream
    pub(crate) fn new(socket: OwnedWriteHalf, registry: Arc<Mutex<CallsRegistry>>) -> Self {
        Self {
            socket: Arc::new(Mutex::new(socket)),
            registry,
        }
    }

    /// Replace writer stream with a new handle if reconnected.
    /// Existing handles will remain valid, and can be used to send data
    pub(crate) async fn on_reconnected(&mut self, other: RpcWriter) {
        trace!("Writer reconnected");

        self.socket = other.socket;

        let mut registry_lock = self.registry.lock().await;
        // Clear pending calls as we're not going to receive responses
        registry_lock.clear_pending_calls();

        debug!("Resending active subscriptions");

        for (message_id, data) in registry_lock.active_subscriptions() {
            debug!("Sending subsription {message_id}: {data:?}");

            let message = RpcMessage {
                id: *message_id,
                data: data.clone(),
            };

            // In case we failed to send immediately send error response
            if let Err(e) = self.socket_write(&message).await {
                warn!("Failed to resent persisten call to a client: {e:?}")
            }
        }
    }

    /// Send one-way mesage
    /// Immediately returns an `Error` if `P` doesn't serialize into Bson, or the client has disconnected
    pub async fn send_message<P: Serialize>(&self, endpoint: &str, data: &P) -> crate::Result<()> {
        let data = bson::to_bson(data).map_err(|e| crate::Error::ParamsTypeError(e.to_string()))?;

        debug!("New message to an `{endpoint}` endpoint: {data:?}");

        let message = RpcMessage {
            id: -1,
            data: message::RpcData::Message {
                endpoint: endpoint.to_owned(),
                body: data,
            },
        };

        if let Err(e) = Box::pin(self.socket_write_and_monitor(&message, true)).await {
            debug!("Error sending a message: {e:?}");

            Err(crate::Error::PeerDisconnected)
        } else {
            Ok(())
        }
    }

    /// Make a client call
    /// Immediately returns an `Error` if `P` doesn't serialize into Bson, or the client has disconnected
    pub async fn call<P: Serialize, R: DeserializeOwned>(
        &self,
        endpoint: &str,
        data: &P,
    ) -> CallResultType<R> {
        let data = bson::to_bson(data).map_err(|e| crate::Error::ParamsTypeError(e.to_string()))?;
        let (id, result) = self.registry.lock().await.add_call();

        debug!("New {id} call to an {endpoint}: {data:?}");

        let message = RpcMessage {
            id,
            data: message::RpcData::Call {
                endpoint: endpoint.to_owned(),
                params: data,
            },
        };

        // In case we failed to send immediately send error response
        if let Err(e) = self.socket_write(&message).await {
            debug!("Error making a call: {e:?}");

            self.registry
                .lock()
                .await
                .resolve(id, Err(crate::Error::PeerDisconnected))
                .await;
        }

        Ok(Box::pin(result.map(|chan_result| {
            match chan_result {
                Ok(data) => data.and_then(|response| match bson::from_bson(response) {
                    Ok(value) => Ok(value),
                    Err(e) => Err(crate::Error::ResultTypeError(e.to_string())),
                }),
                // Channel disconnected
                Err(_) => Err(crate::Error::PeerDisconnected),
            }
        })))
    }

    /// Make a call with FD. Used by the hub to send peer FD's
    /// Immediately returns an `Error` if `P` doesn't serialize into Bson, or the client has disconnected
    pub async fn call_fd<P: Serialize, R: DeserializeOwned>(
        &self,
        endpoint: &str,
        data: &P,
    ) -> CallResultType<(R, UnixStream)> {
        let data = bson::to_bson(data).map_err(|e| crate::Error::ParamsTypeError(e.to_string()))?;
        let (id, result) = self.registry.lock().await.add_fd_call();

        debug!("New {id} FD call to the {endpoint}: {data:?}");

        let message = RpcMessage {
            id,
            data: message::RpcData::Call {
                endpoint: endpoint.to_owned(),
                params: data,
            },
        };

        // In case we failed to send immediately send error response
        if let Err(e) = self.socket_write(&message).await {
            debug!("Error making an FD call: {e:?}");

            self.registry
                .lock()
                .await
                .resolve(id, Err(crate::Error::PeerDisconnected))
                .await;
        }

        Ok(Box::pin(result.map(|chan_result| {
            match chan_result {
                Ok(data) => data.and_then(|response| match bson::from_bson(response.0) {
                    Ok(value) => Ok((value, response.1)),
                    Err(e) => Err(crate::Error::ResultTypeError(e.to_string())),
                }),
                // Channel disconnected
                Err(_) => Err(crate::Error::PeerDisconnected),
            }
        })))
    }

    /// Subscribe to the `endpoint`
    /// Immediately returns an `Error` if the client has disconnected
    pub async fn subscribe<R: DeserializeOwned>(&self, endpoint: &str) -> SubResultType<R> {
        let mut registry_lock = self.registry.lock().await;

        let (id, result) = registry_lock.add_subscription();

        debug!("New subscription with id {id} to the {endpoint}");

        let data = message::RpcData::Subscription {
            endpoint: endpoint.to_owned(),
        };
        let message = RpcMessage {
            id,
            data: data.clone(),
        };

        // Add persistent call to resubscribe on reconnect.
        registry_lock.add_persistent_call(id, data);

        // In case we failed to send immediately send error response
        if let Err(e) = self.socket_write(&message).await {
            debug!("Error subscribing to a client: {e:?}");

            self.registry
                .lock()
                .await
                .resolve(id, Err(crate::Error::PeerDisconnected))
                .await;
        }

        Ok(Box::pin(result.map(|chan_result| {
            chan_result.and_then(|response| match bson::from_bson(response) {
                Ok(value) => Ok(value),
                Err(e) => Err(crate::Error::ResultTypeError(e.to_string())),
            })
        })))
    }

    /// Make a connection request. Blocks until a connection response is received
    /// Immediately returns an `Error` if the client has disconnected
    pub async fn connection_request(
        &self,
        client_name: &str,
        target_name: &str,
        socket: UnixStream,
    ) -> anyhow::Result<()> {
        let message = RpcMessage {
            id: 0,
            data: message::RpcData::ConnectionRequest {
                client_name: client_name.into(),
                target_name: target_name.into(),
            },
        };

        debug!("New connection request from {client_name} to {target_name}");

        if let Err(e) = self.socket_write(&message).await {
            debug!(
                "Failed to send connection request message: {}",
                e.to_string()
            );

            bail!(e);
        }

        if let Err(e) = self.socket.lock().await.send_stream(socket).await {
            debug!(
                "Failed to send connection request socket: {}",
                e.to_string()
            );

            bail!(e);
        }

        Ok(())
    }

    /// Respond to a call
    /// Returns `true` if succesfully responded
    pub async fn respond<P: Serialize>(&self, message_id: i64, data: crate::Result<P>) -> bool {
        let data = data.and_then(|value| {
            bson::to_bson(&value).map_err(|e| crate::Error::ResultTypeError(e.to_string()))
        });

        debug!("Responding to {message_id} with {data:?}");

        let message = RpcMessage {
            id: message_id,
            data: message::RpcData::Response(data),
        };

        if self.socket_write(&message).await.is_err() {
            debug!("Failed to write client response");
            return false;
        }

        true
    }

    /// Respond to a call with FD
    /// Returns `true` if succesfully responded
    pub async fn respond_with_fd<P: Serialize>(
        &self,
        message_id: i64,
        data: crate::Result<P>,
        stream: UnixStream,
    ) -> bool {
        let data = data.and_then(|value| {
            bson::to_bson(&value).map_err(|e| crate::Error::ResultTypeError(e.to_string()))
        });

        debug!("Responding to {message_id} with FD and {data:?}");

        let message = RpcMessage {
            id: message_id,
            data: message::RpcData::FdResponse(data),
        };

        if let Err(e) = self.socket_write(&message).await {
            debug!("Failed to write client response: {}", e.to_string());
            return false;
        } else if let Err(e) = self.socket.lock().await.send_stream(stream).await {
            debug!("Failed to send fd to a client: {}", e.to_string());
            return false;
        }

        true
    }

    async fn socket_write(&self, message: &RpcMessage) -> anyhow::Result<()> {
        self.socket_write_and_monitor(message, false).await
    }

    /// Write message into a socket and monitor
    async fn socket_write_and_monitor(
        &self,
        message: &RpcMessage,
        ignore_monitor: bool,
    ) -> anyhow::Result<()> {
        let result = self.socket.lock().await.write_message(&message).await;

        if !ignore_monitor && result.is_ok() {
            #[cfg(feature = "monitor")]
            {
                use crate::monitor::{Direction, Monitor};
                Monitor::send(message, Direction::Ougoing).await;
            }
        }

        result
    }
}
