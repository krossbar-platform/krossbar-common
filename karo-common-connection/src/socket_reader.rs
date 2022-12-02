use anyhow::{bail, Context, Result};
use bytes::{BufMut, Bytes, BytesMut};
use log::*;
use tokio::io::AsyncReadExt;

/// Read raw Bson from the **socket**
/// **log** can be used for debugging. **Off** by default
pub async fn read_bson_from_socket<S: AsyncReadExt + Unpin>(
    socket: &mut S,
    log: bool,
) -> Result<Bytes> {
    let mut buffer = BytesMut::new();

    // First read Bson length
    let mut bytes_to_read = read_packet_len(socket, log).await?;
    // Push len to buffer so it contains full Bson
    buffer.put_u32_le(bytes_to_read);
    // Substract header len from amount to read
    bytes_to_read -= 4;

    loop {
        // Make a handle to read exact amount of data
        let mut take_handle = socket.take(bytes_to_read as u64);

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
                bytes_to_read = bytes_to_read - bytes_read as u32;
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

                // Try to parse message to take exact amount of data we need to read to get a message
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

/// Read raw Bson len from the [socket] (4 LE bytes at the beginning of the packet)
/// **log** can be used for debugging. **Off** by default
async fn read_packet_len<S: AsyncReadExt + Unpin>(socket: &mut S, log: bool) -> Result<u32> {
    socket
        .read_u32_le()
        .await
        .map(|len| {
            if log {
                trace!("Incoming packet len: {}", len);
            }

            len
        })
        .context("Failed to read incoming packet len")
}
