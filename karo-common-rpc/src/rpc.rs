use std::{
    ops::Deref,
    pin::{pin, Pin},
    sync::Arc,
    task::{Context, Poll},
};

use async_send_fd::AsyncRecvTokioStream as _;
use futures::{lock::Mutex, stream::FusedStream, Future, Stream};
use log::{debug, info, trace, warn};
use tokio::net::{unix::OwnedReadHalf, UnixStream};

use crate::{
    calls_registry::CallsRegistry,
    message::{self, RpcMessage},
    message_stream::AsyncReadMessage,
    request::{Body, RpcRequest},
    writer::{self, RpcWriter},
};

/// RPC handle to a client
pub struct Rpc {
    /// Socket reader to read incoming message
    socket: OwnedReadHalf,
    /// Socker writer handle to send responses
    writer: writer::RpcWriter,
    /// Call registry to resolve incoming responses
    calls_registry: Arc<Mutex<CallsRegistry>>,
}

impl Rpc {
    /// Make new RPC wrapper from [tokio::net::UnixStream]
    pub fn new(stream: UnixStream) -> Self {
        trace!("Making new RPC handle from a stream");

        let calls_registry = Arc::new(Mutex::new(CallsRegistry::new()));
        let (reader, writer) = stream.into_split();

        Self {
            socket: reader,
            writer: RpcWriter::new(writer, calls_registry.clone()),
            calls_registry,
        }
    }

    /// Replace rpc stream with a new handle if reconnected.
    /// Existing subscriptions will be resend to the client.
    /// Pending calls will be discarded
    pub async fn on_reconnected(&mut self, other: Rpc) {
        debug!("RPC reconnected");

        let writer = other.writer().clone();

        self.socket = other.socket;
        self.writer.on_reconnected(writer).await;
    }

    /// Get client writer
    pub fn writer(&self) -> &RpcWriter {
        &self.writer
    }

    /// Poll RPC handle, resolving incoming responses
    async fn poll(&mut self) -> Option<RpcRequest> {
        loop {
            let message: RpcMessage = match self.socket.read_message().await {
                Ok(message) => message,
                Err(e) => {
                    info!(
                        "Failed to read incoming message. Client disconnected?: {}",
                        e.to_string()
                    );
                    return None;
                }
            };

            debug!("Incoming message: {:?}", message);

            match message.data {
                message::RpcData::Call(endpoint, params) => {
                    return Some(RpcRequest::new(
                        message.id,
                        self.writer.clone(),
                        endpoint,
                        Body::Call(params),
                    ));
                }
                message::RpcData::Subscription(endpoint) => {
                    return Some(RpcRequest::new(
                        message.id,
                        self.writer.clone(),
                        endpoint,
                        Body::Subscription,
                    ));
                }
                message::RpcData::ConnectionRequest(service_name) => {
                    match self.socket.as_ref().recv_stream().await {
                        Ok(stream) => {
                            return Some(RpcRequest::new(
                                message.id,
                                self.writer.clone(),
                                "connect".to_owned(),
                                Body::Fd(service_name, stream),
                            ))
                        }
                        Err(_) => warn!("Failed to recieve incoming connection fd"),
                    }
                }
                message::RpcData::Response(body) => {
                    self.calls_registry
                        .lock()
                        .await
                        .resolve(message.id, body)
                        .await
                }
                message::RpcData::FdResponse(body) => match body {
                    Ok(body) => match self.socket.as_ref().recv_stream().await {
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

impl Stream for Rpc {
    type Item = RpcRequest;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        pin!(self.get_mut().poll()).poll(cx)
    }
}

impl FusedStream for Rpc {
    /// Rpc never terminates, because after we've reconnected, we can read it again.
    fn is_terminated(&self) -> bool {
        false
    }
}
