use log::*;
use tokio::{
    io::AsyncWriteExt,
    net::{UnixListener, UnixStream},
    sync::mpsc::{self, Sender},
};

use karo_common_connection::socket_reader::read_bson_from_socket;

pub struct SimpleEchoListener {
    restart_tx: Sender<()>,
}

impl SimpleEchoListener {
    pub async fn new(socket_path: &str) -> Self {
        let (restart_tx, mut restart_rx) = mpsc::channel(5);
        let socket_path: String = socket_path.into();

        tokio::spawn(async move {
            let mut listener = UnixListener::bind(&socket_path).unwrap();

            loop {
                tokio::select! {
                    connection = listener.accept() => {
                        if let Ok((stream, addr)) = connection {
                            debug!("New connection from {:?}", addr);
                            Self::start_echo(stream).await
                        }
                    }
                    restart = restart_rx.recv() => {
                        if restart.is_some() {
                            debug!("Restart request");
                            drop(listener);
                            let _ = std::fs::remove_file(&socket_path);

                            listener = UnixListener::bind(&socket_path).unwrap();
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

    async fn start_echo(mut stream: UnixStream) {
        tokio::spawn(async move {
            loop {
                if let Ok(data) = read_bson_from_socket(&mut stream, true).await {
                    let _ = stream.write_all(&data).await;
                } else {
                    break;
                }
            }
        });
    }

    #[allow(dead_code)]
    pub async fn restart(&mut self) {
        self.restart_tx.send(()).await.unwrap();
    }
}
