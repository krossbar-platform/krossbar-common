use bytes::BytesMut;

use crate::connector::Connector;

pub enum ConnectionState {
    Ready,
    Connecting,
}

pub struct Connection {
    /// Connector to initiate connection and reconnect if disconnected
    connector: Box<dyn Connector>,
    /// Connection state to detect if reconnecting
    state: ConnectionState,
    /// Buffer to read incoming messages
    read_buffer: BytesMut,
}

impl Connection {
    pub fn new(connector: Box<dyn Connector>) -> Self {
        //let stream = connector.connect();
        Self {
            connector,
            state: ConnectionState::Ready,
            read_buffer: BytesMut::with_capacity(64),
        }
    }
}
