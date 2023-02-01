use bson::Bson;
use karo_common_rpc::rpc_connection::RpcConnection;
use log::*;
use tokio::sync::mpsc::{self, Sender};

use karo_common_rpc::Message;

use super::{listen_connector::ListenConnector, message_type};

pub struct SimpleEchoListener {
    restart_tx: Sender<()>,
}

impl SimpleEchoListener {
    pub async fn new(socket_path: &str) -> Self {
        let (restart_tx, mut restart_rx) = mpsc::channel(5);
        let socket_path: String = socket_path.into();

        tokio::spawn(async move {
            let connector = Box::new(ListenConnector::new(&socket_path));
            let mut rpc_client = RpcConnection::new(connector).await.unwrap();

            loop {
                tokio::select! {
                    message = rpc_client.read() => {
                        Self::handle_message(&mut message.unwrap()).await;
                    }
                    restart = restart_rx.recv() => {
                        if restart.is_some() {
                            debug!("Restart request");
                            drop(rpc_client);

                            let connector = Box::new(ListenConnector::new(&socket_path));
                            rpc_client = RpcConnection::new(connector).await.unwrap();
                        } else {
                            info!("Listener dropped. Shutting down");
                            break;
                        }
                    }
                }
            }
        });

        Self { restart_tx }
    }

    async fn handle_message(message: &mut Message) {
        let inmessage = message.body();

        match inmessage {
            message_type::MessageType::Call(bson) => {
                debug!("Received a call '{:?}'. Sending echo", bson);
                assert!(message.is_call());

                message.reply(&message.body::<Bson>()).await.unwrap();
            }
            message_type::MessageType::Subscription(bson) => {
                debug!("Received a subscription '{:?}'. Sending 5 replies", bson);
                assert!(message.is_call());

                for _ in 0..5 {
                    message.reply(&message.body::<Bson>()).await.unwrap();
                }
            }
            message_type::MessageType::Message(bson) => {
                debug!("Received a message '{:?}'. Skipping", bson);
                assert!(!message.is_call());
            }
        }
    }

    pub async fn restart(&mut self) {
        self.restart_tx.send(()).await.unwrap();
    }
}
