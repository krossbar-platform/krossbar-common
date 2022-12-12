use anyhow::{Context, Result};
use bson::Bson;

use karo_common_connection::writer::Writer;

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
}

impl UserMessageHandle {
    /// New one-way message
    pub(crate) fn new(message: Message) -> Self {
        Self {
            needs_response: false,
            response_writer: None,
            message,
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
        }
    }

    pub fn id(&self) -> u64 {
        self.message.seq_no
    }

    pub fn body(&self) -> &Bson {
        &self.message.body
    }

    pub fn is_call(&self) -> bool {
        self.message.message_type == MessageType::Call
    }

    pub async fn reply(&mut self, reply: Bson) -> Result<()> {
        // The writer is Some only if we have a call, and it's the only case when we can reply
        if let Some(writer) = &mut self.response_writer {
            let response = self.message.make_response(reply);
            let bson = bson::to_bson(&response).context("Failed to serialize a reply")?;

            writer
                .write_bson(&bson)
                .await
                .context("Failed to write response into a writer")?;

            // Clear the writer to record response
            self.needs_response = false;

            Ok(())
        } else {
            panic!("Message is not a call, and can't be replied to")
        }
    }
}

impl Drop for UserMessageHandle {
    fn drop(&mut self) {
        if self.needs_response {
            panic!("A call must be replied before dropping the message")
        }
    }
}
