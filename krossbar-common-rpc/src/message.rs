use bson::Bson;
use serde::{Deserialize, Serialize};

/// RPC message
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RpcMessage {
    pub id: i64,
    pub data: RpcData,
}

/// RPC message data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RpcData {
    /// One way message
    Message { endpoint: String, body: Bson },
    /// RPC call
    Call { endpoint: String, params: Bson },
    /// Subscription request
    Subscription { endpoint: String },
    /// Connection request
    /// `client_name` - initiator client name
    /// `target_name` - connection target name, which can be used
    ///     to implement gateways
    ConnectionRequest {
        client_name: String,
        target_name: String,
    },
    /// Message response
    Response(crate::Result<Bson>),
    /// Mesage response, which precedes incoming FD
    FdResponse(crate::Result<Bson>),
}
