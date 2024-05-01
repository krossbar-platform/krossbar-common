use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Error {
    NotAllowed,
    NoEndpoint,
    AlreadyRegistered,
    ServiceNotFound,
    PeerDisconnected,
    ProtocolError,
    ParamsTypeError(String),
    ResultTypeError(String),
    InternalError(String),
    EndpointError(String),
}

pub type Result<T> = std::result::Result<T, Error>;
