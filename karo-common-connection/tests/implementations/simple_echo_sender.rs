use bson::Bson;
use karo_common_connection::connection::Connection;
use log::*;

use super::simple_connector::SimpleConnector;

pub struct SimpleEchoSender {
    connection: Connection,
}

impl SimpleEchoSender {
    pub async fn new(socket_path: &str) -> Self {
        let connector = SimpleConnector::new(socket_path);
        let connection = Connection::new(Box::new(connector), true).await.unwrap();

        Self { connection }
    }

    pub async fn send_receive(&mut self, message: &Bson) -> Bson {
        trace!("Sending data: {:?}", message);

        self.connection.writer().write_bson(message).await.unwrap();

        trace!("Receiving data");
        self.connection.read_bson().await.unwrap()
    }
}
