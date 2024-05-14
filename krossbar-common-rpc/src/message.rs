use bson::Bson;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RpcMessage {
    pub id: i64,
    pub data: RpcData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RpcData {
    Call {
        endpoint: String,
        params: Bson,
    },
    Subscription {
        endpoint: String,
    },
    ConnectionRequest {
        client_name: String,
        target_name: String,
    },
    Response(crate::Result<Bson>),
    FdResponse(crate::Result<Bson>),
}
