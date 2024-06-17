use std::{ops::Deref, os::fd::AsRawFd, sync::Arc};

use async_send_fd::AsyncRecvTokioStream as _;
use futures::lock::Mutex;
#[cfg(not(feature = "log-to-stdout"))]
use log::{debug, info, trace, warn};
use tokio::net::{unix::OwnedReadHalf, UnixStream};

use crate::{
    calls_registry::CallsRegistry,
    message::{self, RpcMessage},
    message_stream::AsyncReadMessage,
    request::{Body, RpcRequest},
    writer::{self, RpcWriter},
};
#[cfg(feature = "log-to-stdout")]
use crate::{debug, info, trace, warn};

/// RPC handle to a client
pub struct Rpc {
    /// Verbose peer name
    peer_name: String,
    /// Socket reader to read incoming message
    socket: OwnedReadHalf,
    /// Socker writer handle to send responses
    writer: writer::RpcWriter,
    /// Call registry to resolve incoming responses
    calls_registry: Arc<Mutex<CallsRegistry>>,
}

impl Rpc {
    /// Make new RPC wrapper from [tokio::net::UnixStream].
    /// `name` is a vebose peer name which is accessible from [RpcWriter] and [RpcRequest]
    pub fn new(stream: UnixStream, peer_name: &str) -> Self {
        trace!("Making new RPC handle from a stream");

        let calls_registry = Arc::new(Mutex::new(CallsRegistry::new()));
        let (reader, writer) = stream.into_split();

        Self {
            peer_name: peer_name.to_owned(),
            socket: reader,
            writer: RpcWriter::new(writer, calls_registry.clone(), peer_name),
            calls_registry,
        }
    }

    /// Verbose peer name
    pub fn peer_name(&self) -> &str {
        &self.peer_name
    }

    /// Replace rpc stream with a new handle if reconnected.
    /// Existing subscriptions will be resend to the client.
    /// Pending calls will be discarded
    pub async fn on_reconnected(&mut self, other: Rpc) {
        let Rpc { socket, writer, .. } = other;

        debug!(
            "RPC reconnected. New socket: <{}>",
            socket.as_ref().as_raw_fd()
        );

        self.socket = socket;
        self.writer.on_reconnected(writer).await;
    }

    /// Get client writer
    pub fn writer(&self) -> &RpcWriter {
        &self.writer
    }

    /// Poll RPC handle, resolving incoming responses
    pub async fn poll(&mut self) -> Option<RpcRequest> {
        loop {
            trace!("Reading data from <{}>", self.socket.as_ref().as_raw_fd());

            let message: RpcMessage = match self.socket.read_message().await {
                Ok(message) => message,
                Err(e) => {
                    info!(
                        "Failed to read incoming message. Client error: {}",
                        e.to_string()
                    );
                    return None;
                }
            };

            debug!("Incoming message: {:?}", message);

            #[cfg(feature = "monitor")]
            {
                use crate::monitor::{Direction, Monitor};
                Monitor::send(&message, Direction::Incoming, &self.peer_name).await;
            }

            match message.data {
                message::RpcData::Message { endpoint, body } => {
                    return Some(RpcRequest::new(
                        -1,
                        self.writer.clone(),
                        endpoint,
                        Body::Message(body),
                    ));
                }
                message::RpcData::Call { endpoint, params } => {
                    return Some(RpcRequest::new(
                        message.id,
                        self.writer.clone(),
                        endpoint,
                        Body::Call(params),
                    ));
                }
                message::RpcData::Subscription { endpoint } => {
                    return Some(RpcRequest::new(
                        message.id,
                        self.writer.clone(),
                        endpoint,
                        Body::Subscription,
                    ));
                }
                message::RpcData::ConnectionRequest {
                    client_name,
                    target_name,
                } => match self.socket.recv_stream().await {
                    Ok(stream) => {
                        return Some(RpcRequest::new(
                            message.id,
                            self.writer.clone(),
                            "connect".to_owned(),
                            Body::Fd {
                                client_name,
                                target_name,
                                stream,
                            },
                        ))
                    }
                    Err(_) => warn!("Failed to recieve incoming connection fd"),
                },
                message::RpcData::Response(body) => {
                    self.calls_registry
                        .lock()
                        .await
                        .resolve(message.id, body)
                        .await
                }
                message::RpcData::FdResponse(body) => match body {
                    Ok(body) => match self.socket.recv_stream().await {
                        Ok(stream) => self.calls_registry.lock().await.resolve_with_fd(
                            message.id,
                            Ok(body),
                            Some(stream),
                        ),
                        Err(_) => self.calls_registry.lock().await.resolve_with_fd(
                            message.id,
                            Err(crate::Error::PeerDisconnected),
                            None,
                        ),
                    },
                    e => self
                        .calls_registry
                        .lock()
                        .await
                        .resolve_with_fd(message.id, e, None),
                },
            }
        }
    }
}

impl Deref for Rpc {
    type Target = RpcWriter;

    fn deref(&self) -> &Self::Target {
        &self.writer
    }
}
