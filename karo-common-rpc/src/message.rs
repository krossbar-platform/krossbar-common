use anyhow::{ensure, Result};
use bson::Bson;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum MessageType {
    Message,
    Call,
    Response,
}

/// A message which is send by a wire
#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    /// Sequential number to resolve calls
    pub seq_no: u64,
    /// Message body
    pub body: Bson,
    /// If right after the message there's a file descriptor.
    /// Should be read right away
    pub has_fd: bool,
    /// Message type
    pub message_type: MessageType,
}

impl Message {
    pub fn new(seq_no: u64, body: Bson, has_fd: bool) -> Self {
        Self {
            seq_no,
            body,
            has_fd,
            message_type: MessageType::Message,
        }
    }

    pub fn new_call(seq_no: u64, body: Bson, has_fd: bool) -> Self {
        Self {
            seq_no,
            body,
            has_fd,
            message_type: MessageType::Call,
        }
    }

    pub fn make_response(&self, body: Bson, has_fd: bool) -> Result<Self> {
        ensure!(
            self.message_type == MessageType::Call,
            "Responses should be onde only out of a call"
        );

        Ok(Self {
            seq_no: self.seq_no,
            body,
            has_fd,
            message_type: MessageType::Response,
        })
    }
}
