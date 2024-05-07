use bson::Bson;
use serde::Serialize;
use tokio::net::UnixStream;

use super::writer::RpcWriter;

/// Incoming message body
pub enum Body {
    /// Method call
    Call(Bson),
    /// Method subscription
    Subscription,
    /// Incoming connection request in a form of UnixStream
    Fd(String, UnixStream),
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

    pub fn writer(&self) -> &RpcWriter {
        &self.writer
    }

    /// Request body. Moves the body out of the request. Can be used only once
    /// All subsequent calls will return `None`
    pub fn take_body(&mut self) -> Option<Body> {
        self.body.take()
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
