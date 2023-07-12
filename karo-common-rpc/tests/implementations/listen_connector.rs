use anyhow::Result;
use async_trait::async_trait;
use log::*;
use tokio::net::{UnixListener, UnixStream};

use karo_common_rpc::rpc_connector::RpcConnector;

pub struct ListenConnector {
    listener: Option<UnixListener>,
    socket_path: String,
}

impl ListenConnector {
    pub fn new(socket_path: &str) -> Self {
        Self {
            listener: Some(UnixListener::bind(&socket_path).unwrap()),
            socket_path: socket_path.into(),
        }
    }
}

#[async_trait]
impl RpcConnector for ListenConnector {
    async fn connect(&self) -> Result<UnixStream> {
        if let Ok((stream, addr)) = self.listener.as_ref().unwrap().accept().await {
            debug!("New connection from {:?}", addr);
            return Ok(stream);
        } else {
            panic!("Should never happen");
        }
    }
}

impl Drop for ListenConnector {
    fn drop(&mut self) {
        debug!(
            "Listen connector dropped. Removing socket file at: {}",
            self.socket_path
        );

        self.listener.take();
        std::fs::remove_file(&self.socket_path).unwrap();
    }
}
