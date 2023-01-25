use karo_common_rpc::{rpc_connection::RpcConnection, rpc_sender::RpcSender};
use log::*;
use tokio::{
    io::AsyncWriteExt,
    net::UnixStream,
    sync::mpsc::{self, Sender},
};

use karo_common_rpc::Message;

use super::{listen_connector::ListenConnector, message_type};

pub struct SimpleEchoFdListener {
    restart_tx: Sender<()>,
}

impl SimpleEchoFdListener {
    pub async fn new(socket_path: &str) -> Self {
        let (restart_tx, mut restart_rx) = mpsc::channel(5);
        let socket_path: String = socket_path.into();

        tokio::spawn(async move {
            let connector = Box::new(ListenConnector::new(&socket_path));
            let mut rpc_client = RpcConnection::new(connector, true).await.unwrap();

            loop {
                tokio::select! {
                    message = rpc_client.read() => {
                        Self::handle_message(&mut message.unwrap(), rpc_client.sender()).await;
                    }
                    restart = restart_rx.recv() => {
                        if restart.is_some() {
                            debug!("Restart request");
                            drop(rpc_client);

                            let connector = Box::new(ListenConnector::new(&socket_path));
                            rpc_client = RpcConnection::new(connector, true).await.unwrap();
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

    async fn handle_message(message: &mut Message, mut sender: RpcSender) {
        let inmessage =
            bson::from_bson::<message_type::MessageType>(message.body().clone()).unwrap();

        match inmessage {
            message_type::MessageType::Call(bson) => {
                debug!("Received a call '{:?}'. Sending echo", bson);
                assert!(message.is_call());

                // Reply with a file descriptor
                let (reader, mut writer) = UnixStream::pair().unwrap();

                message.reply_with_fd(bson.clone(), reader).await.unwrap();
                writer
                    .write_all(&bson::to_raw_document_buf(&bson).unwrap().into_bytes())
                    .await
                    .unwrap();
            }
            message_type::MessageType::Subscription(_) => {
                unreachable!();
            }
            message_type::MessageType::Message(bson) => {
                debug!("Received a message '{:?}'. Sending fd message", bson);
                assert!(!message.is_call());

                if let Some(stream) = message.take_fd() {
                    sender.send_fd(bson, stream).await.unwrap();
                } else {
                    panic!("User message must contain a descriptor");
                }
            }
        }
    }

    pub async fn restart(&mut self) {
        self.restart_tx.send(()).await.unwrap();
    }
}
