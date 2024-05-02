use bson::Bson;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct RpcMessage {
    pub id: i64,
    pub data: RpcData,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum RpcData {
    Call(String, Bson),
    Subscribtion(String),
    ConnectionRequest(String),
    Response(crate::Result<Bson>),
    FdResponse(crate::Result<Bson>),
}
