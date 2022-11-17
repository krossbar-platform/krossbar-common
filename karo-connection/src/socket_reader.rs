use anyhow::{bail, Result};
use bytes::{Bytes, BytesMut};
use log::*;
use tokio::io::AsyncReadExt;

pub async fn read_bson_from_socket<S: AsyncReadExt + Unpin>(
    socket: &mut S,
    log: bool,
) -> Result<Bytes> {
    let mut buffer = BytesMut::new();

    // First read Bson length
    let mut bytes_to_read = 4;

    loop {
        // Make a handle to read exact amount of data
        let mut take_handle = socket.take(bytes_to_read);

        match take_handle.read_buf(&mut buffer).await {
            Ok(bytes_read) => {
                // Socket closed
                if bytes_read == 0 {
                    if log {
                        trace!("Read zero bytes from a socket");
                    }

                    bail!("Socket closed");
                }

                // Descrease bytes by number of bytes already read
                bytes_to_read = bytes_to_read - bytes_read as u64;
                if log {
                    trace!(
                        "Read {} bytes from socket. Still {} to read",
                        bytes_read,
                        bytes_to_read
                    );
                }

                // Still need more data to read
                if bytes_to_read != 0 {
                    continue;
                }

                return Ok(buffer.into());
            }
            Err(err) => {
                if log {
                    error!(
                    "Failed to read from a socket: {}. Client is disconnected. Shutting him down",
                    err.to_string()
                );
                }
                bail!(err);
            }
        };
    }
}
