use bson::Bson;
use bytes::Bytes;
use karo_connection::connection::Connection;

use super::simple_connector::SimpleConnector;

pub struct SimpleEchoSender {
    connection: Connection,
}

impl SimpleEchoSender {
    pub async fn new(socket_path: &str) -> Self {
        let connector = SimpleConnector::new(socket_path);
        let connection = Connection::new(Box::new(connector)).await.unwrap();

        Self { connection }
    }

    pub async fn send_receive(&mut self, message: &Bson) -> Bson {
        println!("Sending data: {:?}", message);
        let send_data = Bytes::from(bson::to_raw_document_buf(message).unwrap().into_bytes());
        self.connection.writer().write(send_data).await.unwrap();

        println!("Receiving data");
        let receive_data = self.connection.read().await.unwrap();
        bson::from_slice(&receive_data).unwrap()
    }
}
