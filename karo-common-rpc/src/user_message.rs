use std::fmt::Debug;

use anyhow::{Context, Result};
use bson::Bson;

use karo_common_connection::writer::Writer;
use serde::{de::DeserializeOwned, Serialize};
use tokio::net::UnixStream;

use crate::message::{Message, MessageType};

/// Incoming RPC message returned to a user as a result of reading from the connection
pub struct UserMessageHandle {
    /// Every call should be responded at least once before dropping the message.
    /// Destructor panics otherwise
    needs_response: bool,
    /// Writer to send response into if a call
    response_writer: Option<Writer>,
    /// Incoming internal message body
    message: Message,
    /// Optional file descriptor
    fd: Option<UnixStream>,
}

impl UserMessageHandle {
    /// New one-way message
    pub(crate) fn new(message: Message, fd: Option<UnixStream>) -> Self {
        Self {
            needs_response: false,
            response_writer: None,
            message,
            fd,
        }
    }

    /// New call
    pub(crate) fn new_call(message: Message, response_writer: Writer) -> Self {
        assert!(
            message.message_type == MessageType::Call,
            "Trying to create a call handle fron a non-call message"
        );

        Self {
            needs_response: true,
            response_writer: Some(response_writer),
            message,
            fd: None,
        }
    }

    pub fn id(&self) -> u64 {
        self.message.seq_no
    }

    pub fn body<M: DeserializeOwned>(&self) -> M {
        bson::from_bson(self.message.body.clone()).unwrap()
    }

    pub fn take_fd(&mut self) -> Option<UnixStream> {
        self.fd.take()
    }

    pub fn is_call(&self) -> bool {
        self.message.message_type == MessageType::Call
    }

    pub async fn reply<M: Serialize>(&mut self, reply: &M) -> Result<()> {
        let body = bson::to_bson(reply).unwrap();

        // The writer is Some only if we have a call, and it's the only case when we can reply
        if let Some(writer) = &mut self.response_writer {
            let response = self.message.make_response(body, false)?;
            let bson = bson::to_bson(&response).context("Failed to serialize a reply")?;

            writer
                .write_bson(&bson)
                .await
                .context("Failed to write response into a writer")?;

            // To not panic into the destructor
            self.needs_response = false;

            Ok(())
        } else {
            panic!("Message is not a call, and can't be replied to")
        }
    }

    pub async fn reply_with_fd(&mut self, reply: Bson, stream: UnixStream) -> Result<()> {
        // The writer is Some only if we have a call, and it's the only case when we can reply
        if let Some(writer) = &mut self.response_writer {
            let response = self.message.make_response(reply, true)?;
            let bson = bson::to_bson(&response).context("Failed to serialize a reply")?;

            writer
                .write_bson_fd(&bson, stream)
                .await
                .context("Failed to write response into a writer")?;

            // To not panic into the destructor
            self.needs_response = false;

            Ok(())
        } else {
            panic!("Message is not a call, and can't be replied to")
        }
    }
}

impl Debug for UserMessageHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Message {{ need_response: {}, message: {:?} }}",
            self.needs_response, self.message
        )
    }
}

impl Drop for UserMessageHandle {
    fn drop(&mut self) {
        if self.needs_response {
            panic!("A call must be replied before dropping the message")
        }
    }
}
