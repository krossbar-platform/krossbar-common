use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize, Clone, Error)]
pub enum Error {
    /// User is not allowed to register with a provided service name, or
    /// not allowed to connect to a requested client
    #[error("Connection or service name is not allowed")]
    NotAllowed,
    /// Not such endpoint to call or subscribe to
    #[error("Requested endpoint is not registered")]
    NoEndpoint,
    /// Service or endpoind had been registered already
    #[error("Service or endpoint is already registered")]
    AlreadyRegistered,
    /// Not found a requested service
    #[error("Requested service is not found")]
    ServiceNotFound,
    /// Peer disconnected
    #[error("Peer disconencted")]
    PeerDisconnected,
    /// Invalid params to a call. Contains deserialization error
    #[error("Invalid call params: {0}")]
    ParamsTypeError(String),
    /// Invalid result type of an enpoint requested. Either call result type, or
    /// subscription result type. COntains deserialization error
    #[error("Invalid result type: {0}")]
    ResultTypeError(String),
    /// Internal library error. Should never happen
    #[error("Internal Krossbar error: {0}. Please report the issue")]
    InternalError(String),
    /// Client error
    #[error("Client returned an error: {0}")]
    ClientError(String),
}

pub type Result<T> = std::result::Result<T, Error>;
