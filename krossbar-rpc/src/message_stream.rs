use std::io::Cursor;

use bson::Document;
use log::trace;
use serde::{de::DeserializeOwned, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// A trait which can read [serde::de::DeserializeOwned] from a stream
pub trait AsyncReadMessage<T: DeserializeOwned> {
    async fn read_message(&mut self) -> crate::Result<T>;
}

/// A trait which can write [serde::ser::Serialize] into a stream
pub trait AsyncWriteMessage<T: Serialize> {
    async fn write_message(&mut self, message: &T) -> crate::Result<()>;
}

impl<R, T> AsyncReadMessage<T> for R
where
    R: AsyncReadExt + Unpin,
    T: DeserializeOwned,
{
    async fn read_message(&mut self) -> crate::Result<T> {
        // Read BSON len
        let mut len_buf = [0u8; 4];

        self.read_exact(&mut len_buf)
            .await
            .map_err(|_| crate::Error::PeerDisconnected)?;

        let len = i32::from_le_bytes(len_buf);
        trace!("BSON message len: {:?}", len);

        // Read BSON body. Prepend BSON len to the rest of the data
        let mut data: Vec<u8> = len_buf.into();
        self.take((len - 4) as u64)
            .read_to_end(&mut data)
            .await
            .map_err(|_| crate::Error::PeerDisconnected)?;

        let mut cursor = Cursor::new(data);
        let doc = Document::from_reader(&mut cursor)
            .map_err(|e| crate::Error::InternalError(e.to_string()))?;

        Ok(bson::from_document(doc).map_err(|e| crate::Error::InternalError(e.to_string()))?)
    }
}

impl<W, T> AsyncWriteMessage<T> for W
where
    W: AsyncWriteExt + Unpin,
    T: Serialize,
{
    async fn write_message(&mut self, message: &T) -> crate::Result<()> {
        let doc =
            bson::to_document(message).map_err(|e| crate::Error::InternalError(e.to_string()))?;

        let mut buffer: Vec<u8> = Vec::new();
        doc.to_writer(&mut buffer)
            .map_err(|e| crate::Error::InternalError(e.to_string()))?;

        self.write_all(&buffer)
            .await
            .map_err(|_| crate::Error::PeerDisconnected)?;
        Ok(())
    }
}
