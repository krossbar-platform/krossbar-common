use std::fmt::{Debug, Formatter, Result as FmtResult};

use bson::Bson;
use serde::Serialize;
use tokio::net::UnixStream;

use super::writer::RpcWriter;

/// Incoming message body
#[derive(Debug)]
pub enum Body {
    /// One way message
    Message(Bson),
    /// Method call
    Call(Bson),
    /// Method subscription
    Subscription,
    /// Incoming connection request in a form of UnixStream
    Fd {
        client_name: String,
        target_name: String,
        stream: UnixStream,
    },
}

/// Client request
pub struct RpcRequest {
    /// Message id from the client
    message_id: i64,
    /// Writer to repond to the message
    writer: RpcWriter,
    /// Requested endpoint name
    endpoint: String,
    /// Body. It's an option to allow user to steal body data
    body: Option<Body>,
}

impl RpcRequest {
    pub(crate) fn new(message_id: i64, writer: RpcWriter, endpoint: String, body: Body) -> Self {
        Self {
            message_id,
            writer,
            endpoint,
            body: Some(body),
        }
    }

    pub fn message_id(&self) -> i64 {
        self.message_id
    }

    /// Writer to write response into
    pub fn writer(&self) -> &RpcWriter {
        &self.writer
    }

    /// Verbose peer name
    pub fn peer_name(&self) -> &str {
        self.writer.peer_name()
    }

    /// Request body. Moves the body out of the request. Can be used only once
    /// All subsequent calls will return `None`
    pub fn take_body(&mut self) -> Option<Body> {
        self.body.take()
    }

    /// Peek request body
    pub fn body(&self) -> &Option<Body> {
        &self.body
    }

    /// Requested endpoint
    pub fn endpoint(&self) -> &String {
        &self.endpoint
    }

    /// Respond to the call
    pub async fn respond<T: Serialize>(&self, data: Result<T, crate::Error>) -> bool {
        self.writer.respond(self.message_id, data).await
    }

    /// Respond with FD
    pub async fn respond_with_fd<T: Serialize>(
        &self,
        data: Result<T, crate::Error>,
        stream: UnixStream,
    ) -> bool {
        self.writer
            .respond_with_fd(self.message_id, data, stream)
            .await
    }
}

impl Debug for RpcRequest {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "RpcRequest {{ message_id: {}, endpoint: \"{}\", body: {:?} }}",
            self.message_id, self.endpoint, self.body
        )
    }
}
