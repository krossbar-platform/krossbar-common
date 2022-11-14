use bytes::Bytes;
use tokio::sync::mpsc::Sender;

#[derive(Clone)]
pub struct Writer {
    sender: Sender<Bytes>,
}

impl Writer {
    pub fn new(sender: Sender<Bytes>) -> Self {
        Self { sender }
    }

    pub async fn write(&mut self, bytes: Bytes) -> Option<()> {
        self.sender.send(bytes).await.ok()
    }
}
