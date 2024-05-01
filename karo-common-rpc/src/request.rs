use bson::Bson;
use serde::Serialize;
use tokio::net::UnixStream;

use super::writer::RpcWriter;

/// Client request
pub struct RpcRequest {
    /// Message id from the client
    message_id: i64,
    /// Writer to repond to the message
    writer: RpcWriter,
    /// Requested endpoint name
    endpoint: String,
    /// Endpoint call params
    params: Bson,
    /// Optional FD, which user can send to the client
    stream: Option<UnixStream>,
}

impl RpcRequest {
    pub(crate) fn new(
        message_id: i64,
        writer: RpcWriter,
        endpoint: String,
        params: Bson,
        stream: Option<UnixStream>,
    ) -> Self {
        Self {
            message_id,
            writer,
            endpoint,
            params,
            stream,
        }
    }

    pub fn message_id(&self) -> i64 {
        self.message_id
    }

    pub fn params(&self) -> &Bson {
        &self.params
    }

    pub fn endpoint(&self) -> &String {
        &self.endpoint
    }

    pub fn writer(&self) -> &RpcWriter {
        &self.writer
    }

    pub fn stream(&mut self) -> &mut Option<UnixStream> {
        &mut self.stream
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
