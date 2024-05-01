use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Error {
    /// User is not allowed to register with a provided service name, or
    /// not allowed to connect to a requested client
    NotAllowed,
    /// Not such endpoint to call or subscribe to
    NoEndpoint,
    /// Service or endpoind had been registered already
    AlreadyRegistered,
    /// Not found a requested service
    ServiceNotFound,
    /// Peer disconnected
    PeerDisconnected,
    /// Internal protocol error. Ideally should never occur
    ProtocolError,
    /// Invalid params to a call. Contains deserialization error
    ParamsTypeError(String),
    /// Invalid result type of an enpoint requested. Either call result type, or
    /// subscription result type. COntains deserialization error
    ResultTypeError(String),
    /// Internal library error. Should never happen
    InternalError(String),
    /// Client endpoint error
    EndpointError(String),
}

pub type Result<T> = std::result::Result<T, Error>;
