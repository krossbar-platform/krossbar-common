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
    pub seq_no: u64,
    pub body: Bson,
    pub message_type: MessageType,
}

impl Message {
    pub fn new(seq_no: u64, body: Bson) -> Self {
        Self {
            seq_no,
            body,
            message_type: MessageType::Message,
        }
    }

    pub fn new_call(seq_no: u64, body: Bson) -> Self {
        Self {
            seq_no,
            body,
            message_type: MessageType::Call,
        }
    }

    pub fn make_response(&self, body: Bson) -> Result<Self> {
        ensure!(
            self.message_type == MessageType::Call,
            "Responses should be onde only out of a call"
        );

        Ok(Self {
            seq_no: self.seq_no,
            body,
            message_type: MessageType::Response,
        })
    }
}
