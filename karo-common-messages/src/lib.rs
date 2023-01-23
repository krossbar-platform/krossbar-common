use std::fmt::Display;

use bson::{self, Bson};
use serde::{Deserialize, Serialize};

mod errors;

pub const PROTOCOL_VERSION: i64 = 1;

/// Message body, which constains sent data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    /// Hub messages
    HubMessage(HubMessage),
    /// Peer messages
    PeerMessage(PeerMessage),
    /// Message response (for both hub messages and client calls)
    Response(Response),
}

impl Message {
    pub fn new_registration(service_name: String) -> Self {
        Message::HubMessage(HubMessage::Register {
            protocol_version: PROTOCOL_VERSION,
            service_name,
        })
    }

    pub fn new_connection(peer_service_name: String, await_connection: bool) -> Self {
        Message::HubMessage(HubMessage::Connect {
            peer_service_name,
            await_connection,
        })
    }

    pub fn new_call<T: Serialize>(caller_name: String, method_name: String, data: &T) -> Self {
        Message::PeerMessage(PeerMessage::MethodCall {
            caller_name,
            method_name,
            params: bson::to_bson(&data).unwrap(),
        })
    }

    pub fn new_subscription(subscriber_name: String, signal_name: String) -> Self {
        Message::PeerMessage(PeerMessage::SignalSubscription {
            subscriber_name,
            signal_name,
        })
    }

    pub fn new_watch(subscriber_name: String, state_name: String) -> Self {
        Message::PeerMessage(PeerMessage::StateSubscription {
            subscriber_name,
            state_name,
        })
    }
}

impl Into<Bson> for Message {
    fn into(self) -> Bson {
        bson::to_bson(&self).unwrap()
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HubMessage(service_message) => write!(f, "{}", service_message),
            Self::PeerMessage(peer_message) => write!(f, "{}", peer_message),
            Self::Response(response) => write!(f, "{}", response),
        }
    }
}

/// Hub operated message
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum HubMessage {
    /// Client sends message to Hub to resister with *service_name*
    Register {
        protocol_version: i64,
        service_name: String,
    },
    /// Client sends message to Hub to connect to *peer_service_name*
    /// If **await_connection** hub will be waiting for service to start
    Connect {
        peer_service_name: String,
        await_connection: bool,
    },
}

impl Display for HubMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Register {
                protocol_version,
                service_name,
            } => write!(
                f,
                "Registration request. Protocol version {}. Requested service name: '{}'",
                protocol_version, service_name
            ),
            Self::Connect {
                peer_service_name,
                await_connection,
            } => write!(
                f,
                "Connection request to '{}'. Will await?: {}",
                peer_service_name, await_connection
            ),
        }
    }
}

impl Into<Message> for HubMessage {
    fn into(self) -> Message {
        Message::HubMessage(self)
    }
}

/// Peer call messages
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PeerMessage {
    /// Outgoing method call
    MethodCall {
        caller_name: String,
        method_name: String,
        params: Bson,
    },
    /// Subscriptions request
    SignalSubscription {
        subscriber_name: String,
        signal_name: String,
    },
    /// State subscription request
    StateSubscription {
        subscriber_name: String,
        state_name: String,
    },
}

impl Display for PeerMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MethodCall {
                caller_name: _,
                method_name,
                params,
            } => write!(
                f,
                "Method '{}' call. Argument value: {}",
                method_name, params
            ),
            Self::SignalSubscription {
                subscriber_name: _,
                signal_name,
            } => write!(f, "Signal '{}' subscription request", signal_name),
            Self::StateSubscription {
                subscriber_name: _,
                state_name,
            } => write!(f, "State '{}' watch request", state_name),
        }
    }
}

impl Into<Message> for PeerMessage {
    fn into(self) -> Message {
        Message::PeerMessage(self)
    }
}

/// Call reponses. Including state changes and signal emissions
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Response {
    /// Ok responce. Usually as a response to a successful subscribtion
    Ok,
    /// If one Client performs connection, other Client receives this message to make
    /// p2p connection. Right after the messge we need to read peer UDS file descriptor
    IncomingPeerFd(String),
    /// Client sends shutdown message when is going to disconnect
    Shutdown(String),
    /// Method call result
    Return(Bson),
    /// Emitted signal
    Signal(Bson),
    /// Emitted state change
    StateChanged(Bson),
    /// An error
    Error(errors::Error),
}

impl Into<Message> for Response {
    fn into(self) -> Message {
        Message::Response(self)
    }
}

impl Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ok => write!(f, "Ok"),
            Self::Shutdown(reason) => write!(f, "Shutdown message. Reason: {}", reason),
            Self::Return(bson) => write!(f, "Method response: {}", bson),
            Self::Signal(bson) => write!(f, "Signal emitted: {}", bson),
            Self::StateChanged(bson) => write!(f, "State changed: {}", bson),
            Self::Error(err) => write!(f, "Error: {}", err),
            Self::IncomingPeerFd(peer_service_name) => {
                write!(f, "Incoming FD for a peer '{}'", peer_service_name)
            }
        }
    }
}
