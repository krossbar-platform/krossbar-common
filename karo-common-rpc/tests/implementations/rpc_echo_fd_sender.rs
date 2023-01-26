use bson::Bson;
use karo_common_rpc::{rpc_connection::RpcConnection, rpc_sender::OneReceiver, Message};
use log::*;
use tokio::net::UnixStream;

use crate::implementations::message_type::MessageType;

use super::simple_connector::SimpleConnector;

pub struct SimpleEchoFdSender {
    connection: RpcConnection,
}

impl SimpleEchoFdSender {
    pub async fn new(socket_path: &str) -> Self {
        let connector = SimpleConnector::new(socket_path);
        let connection = RpcConnection::new(Box::new(connector)).await.unwrap();

        Self { connection }
    }

    pub async fn start_loop(mut self) {
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    message = self.connection.read() => {
                        unreachable!("Client shold never receive a message. Got: {:?}", message);
                    }
                }
            }
        });
    }

    pub async fn read(&mut self) -> Message {
        self.connection.read().await.unwrap()
    }

    pub async fn send(&mut self, message: &Bson) {
        debug!("Sending message: {:?}", message);

        let bson = bson::to_bson(&MessageType::Message(message.clone())).unwrap();

        self.connection.sender().send(bson).await.unwrap();
    }

    pub async fn send_fd(&mut self, message: &Bson, stream: UnixStream) {
        debug!("Sending message: {:?}", message);

        let bson = bson::to_bson(&MessageType::Message(message.clone())).unwrap();

        self.connection
            .sender()
            .send_fd(bson, stream)
            .await
            .unwrap();
    }

    pub async fn call(&mut self, message: &Bson) -> OneReceiver<Message> {
        debug!("Call: {:?}", message);

        let bson = bson::to_bson(&MessageType::Call(message.clone())).unwrap();

        self.connection.sender().call(bson).await.unwrap()
    }
}
