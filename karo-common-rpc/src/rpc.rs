use std::{ops::Deref, sync::Arc};

use async_send_fd::AsyncRecvTokioStream as _;
use bson::Bson;
use futures::lock::Mutex;
use log::{debug, warn};
use tokio::net::{unix::OwnedReadHalf, UnixStream};

use crate::{
    calls_registry::CallsRegistry,
    message::{self, RpcMessage},
    message_stream::AsyncReadMessage,
    request::RpcRequest,
    writer::{self, RpcWriter},
};

pub struct Rpc {
    socket: OwnedReadHalf,
    writer: writer::RpcWriter,
    calls_registry: Arc<Mutex<CallsRegistry>>,
}

impl Rpc {
    pub fn new(stream: UnixStream) -> Self {
        let calls_registry = Arc::new(Mutex::new(CallsRegistry::new()));
        let (reader, writer) = stream.into_split();

        Self {
            socket: reader,
            writer: RpcWriter::new(writer, calls_registry.clone()),
            calls_registry,
        }
    }

    pub fn replace(&mut self, other: Rpc) {
        let writer = other.writer().clone();

        self.socket = other.socket;
        self.writer.replace(writer);
    }

    pub fn writer(&self) -> &RpcWriter {
        &self.writer
    }

    pub async fn poll(&mut self) -> Option<RpcRequest> {
        loop {
            let message: RpcMessage = self.socket.read_message().await.ok()?;

            debug!("Incoming message: {:?}", message);

            match message.data {
                message::RpcData::Call(endpoint, params) => {
                    return Some(RpcRequest::new(
                        message.id,
                        self.writer.clone(),
                        endpoint,
                        params,
                        None,
                    ));
                }
                message::RpcData::ConnectionRequest(service_name) => {
                    match self.socket.as_ref().recv_stream().await {
                        Ok(stream) => {
                            return Some(RpcRequest::new(
                                message.id,
                                self.writer.clone(),
                                "connect".to_owned(),
                                Bson::String(service_name),
                                Some(stream),
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
