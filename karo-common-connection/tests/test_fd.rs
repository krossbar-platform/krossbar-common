use bson::{bson, Bson};
use bytes::BytesMut;
use log::*;
use tempdir::TempDir;

mod implementations;

use implementations::{
    simple_echo_fd_listener::SimpleEchoFdListener, simple_echo_sender::SimpleEchoSender,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};

#[tokio::test(flavor = "multi_thread")]
async fn test_send_fd() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Trace)
        .try_init();

    let socket_dir = TempDir::new("karo_hub_socket_dir").expect("Failed to create socket tempdir");
    let socket_path: String = socket_dir
        .path()
        .join("karo_hub.socket")
        .as_os_str()
        .to_str()
        .unwrap()
        .into();

    let mut listener = SimpleEchoFdListener::new(&socket_path).await;
    let mut connection = SimpleEchoSender::new(&socket_path).await;

    let bson = bson!({
        "message": "Hello world!"
    });

    let (mut send_stream, receive_stream) = UnixStream::pair().unwrap();

    let (bson_received, mut stream_received) =
        connection.send_receive_fd(&bson, receive_stream).await;

    debug!("Sent data: {}, received data: {}", bson, bson_received);

    assert_eq!(bson_received, bson);

    // Send message through the received stream
    {
        send_stream
            .write_all(&bson::to_raw_document_buf(&bson).unwrap().into_bytes())
            .await
            .unwrap();

        let mut buffer = BytesMut::new();
        stream_received.read_buf(&mut buffer).await.unwrap();
        let pair_received_bson = bson::from_slice::<Bson>(&buffer).unwrap();

        debug!(
            "Socket sent data: {}, received data: {}",
            bson, pair_received_bson
        );

        assert_eq!(pair_received_bson, bson);
    }

    listener.restart().await;

    // Send message through the received stream 2
    {
        send_stream
            .write_all(&bson::to_raw_document_buf(&bson).unwrap().into_bytes())
            .await
            .unwrap();

        let mut buffer = BytesMut::new();
        stream_received.read_buf(&mut buffer).await.unwrap();
        let pair_received_bson = bson::from_slice::<Bson>(&buffer).unwrap();

        debug!(
            "2 Socket sent data: {}, received data: {}",
            bson, pair_received_bson
        );

        assert_eq!(pair_received_bson, bson);
    }
}
