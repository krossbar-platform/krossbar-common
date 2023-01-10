use bson::Bson;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum MessageType {
    Message(Bson),
    Call(Bson),
    Subscription(Bson),
}
