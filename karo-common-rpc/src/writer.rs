use std::{pin::Pin, sync::Arc};

use anyhow::bail;
use async_send_fd::AsyncSendTokioStream;
use futures::{lock::Mutex, stream::FusedStream, Future, FutureExt as _, StreamExt as _};
use log::{debug, info, warn};
use serde::{de::DeserializeOwned, Serialize};
use tokio::net::{unix::OwnedWriteHalf, UnixStream};

use crate::message_stream::AsyncWriteMessage;

use super::{
    calls_registry::CallsRegistry,
    message::{self, RpcMessage},
};

type CallResultType<T> = crate::Result<Pin<Box<dyn Future<Output = crate::Result<T>> + Send>>>;
type SubResultType<T> = crate::Result<Pin<Box<dyn FusedStream<Item = crate::Result<T>> + Send>>>;

#[derive(Clone)]
pub struct RpcWriter {
    socket: Arc<Mutex<OwnedWriteHalf>>,
    registry: Arc<Mutex<CallsRegistry>>,
}

impl RpcWriter {
    pub fn new(socket: OwnedWriteHalf, registry: Arc<Mutex<CallsRegistry>>) -> Self {
        Self {
            socket: Arc::new(Mutex::new(socket)),
            registry,
        }
    }

    pub fn replace(&mut self, other: RpcWriter) {
        self.socket = other.socket;
    }

    pub async fn call<P: Serialize, R: DeserializeOwned>(
        &self,
        endpoint: String,
        data: &P,
    ) -> CallResultType<R> {
        let data = bson::to_bson(data).map_err(|e| crate::Error::ParamsTypeError(e.to_string()))?;
        let (id, result) = self.registry.lock().await.add_call();

        let message = RpcMessage {
            id,
            data: message::RpcData::Call(endpoint, data),
        };

        debug!("Call: {:?}", message);

        // In case we failed to send immediately send error response
        if let Err(e) = self.socket.lock().await.write_message(&message).await {
            debug!("Error making a call: {e:?}");

            self.registry
                .lock()
                .await
                .resolve(id, Err(crate::Error::PeerDisconnected))
                .await;
        }

        debug!("Succesfully written a message");

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

    pub async fn call_fd<P: Serialize, R: DeserializeOwned>(
        &self,
        endpoint: String,
        data: &P,
    ) -> CallResultType<(R, UnixStream)> {
        let data = bson::to_bson(data).map_err(|e| crate::Error::ParamsTypeError(e.to_string()))?;
        let (id, result) = self.registry.lock().await.add_fd_call();

        let message = RpcMessage {
            id,
            data: message::RpcData::Call(endpoint, data),
        };

        debug!("Call fd: {:?}", message);

        // In case we failed to send immediately send error response
        if let Err(_) = self.socket.lock().await.write_message(&message).await {
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

    pub async fn subscribe<P: Serialize, R: DeserializeOwned>(
        &self,
        endpoint: String,
        data: &P,
    ) -> SubResultType<R> {
        let data = bson::to_bson(data).map_err(|e| crate::Error::ParamsTypeError(e.to_string()))?;
        let (id, result) = self.registry.lock().await.add_subscription();

        let message = RpcMessage {
            id,
            data: message::RpcData::Call(endpoint, data),
        };

        debug!("Subscription: {:?}", message);

        // In case we failed to send immediately send error response
        if let Err(_) = self.socket.lock().await.write_message(&message).await {
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

    pub async fn connection_request(
        &self,
        service_name: &String,
        socket: UnixStream,
    ) -> anyhow::Result<()> {
        let message = RpcMessage {
            id: 0,
            data: message::RpcData::ConnectionRequest(service_name.clone()),
        };

        debug!("Connection request: {:?}", service_name);

        let mut socket_lock = self.socket.lock().await;

        if let Err(e) = socket_lock.write_message(&message).await {
            warn!(
                "Failed to send connection request message: {}",
                e.to_string()
            );

            bail!(e);
        }

        if let Err(e) = socket_lock.as_ref().send_stream(socket).await {
            warn!(
                "Failed to send connection request socket: {}",
                e.to_string()
            );

            bail!(e);
        }

        Ok(())
    }

    pub async fn respond<P: Serialize>(&self, message_id: i64, data: crate::Result<P>) -> bool {
        let data = data.and_then(|value| {
            bson::to_bson(&value).map_err(|e| crate::Error::ResultTypeError(e.to_string()))
        });

        let message = RpcMessage {
            id: message_id,
            data: message::RpcData::Response(data),
        };

        debug!("Response: {:?}", message);

        if let Err(_) = self.socket.lock().await.write_message(&message).await {
            info!("Failed to srite client response");
            return false;
        }

        return true;
    }

    pub async fn respond_with_fd<P: Serialize>(
        &self,
        message_id: i64,
        data: crate::Result<P>,
        stream: UnixStream,
    ) -> bool {
        let data = data.and_then(|value| {
            bson::to_bson(&value).map_err(|e| crate::Error::ResultTypeError(e.to_string()))
        });

        let message = RpcMessage {
            id: message_id,
            data: message::RpcData::FdResponse(data),
        };

        debug!("Response with fd: {:?}", message);

        let mut lock = self.socket.lock().await;

        if let Err(_) = lock.write_message(&message).await {
            info!("Failed to write client response");
            return false;
        } else {
            if let Err(_) = lock.as_ref().send_stream(stream).await {
                info!("Failed to send fd to a client");
                return false;
            }
        }

        return true;
    }
}
