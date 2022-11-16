use tokio::{
    io::AsyncWriteExt,
    net::{UnixListener, UnixStream},
    sync::mpsc::{self, Sender},
};

use karo_connection::socket_reader::read_bson_from_socket;

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
                            println!("New connection from {:?}", addr);
                            Self::start_echo(stream).await
                        }
                    }
                    _ = restart_rx.recv() => {
                        println!("Restart request");
                        drop(listener);
                        listener = UnixListener::bind(&socket_path).unwrap();
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
                    println!("New Bson");
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
