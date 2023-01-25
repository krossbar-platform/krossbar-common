use bson::{bson, Bson};
use bytes::BytesMut;
use log::*;
use tempdir::TempDir;

mod implementations;

use implementations::{
    rpc_echo_fd_listener::SimpleEchoFdListener, rpc_echo_fd_sender::SimpleEchoFdSender,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};

#[tokio::test(flavor = "multi_thread")]
async fn test_rpc_fd() {
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

    let _listener = SimpleEchoFdListener::new(&socket_path).await;

    let mut sender = SimpleEchoFdSender::new(&socket_path).await;

    let message = bson!({
        "message": "Hello world!"
    });

    // First we create a socket pair. We write into one end, and read from the other end after it's been echoed
    let (reader, mut writer) = UnixStream::pair().unwrap();

    writer
        .write_all(&bson::to_raw_document_buf(&message).unwrap().into_bytes())
        .await
        .unwrap();

    // See a message with a descriptor. It will be received back in th enext message
    sender.send_fd(&message, reader).await;

    // Received fd is basically our echoed **reader**
    let mut response_message = sender.read().await;

    let mut stream_received = response_message.take_fd().unwrap();
    debug!("Message response: {}", response_message.body());

    let mut buffer = BytesMut::new();
    stream_received.read_buf(&mut buffer).await.unwrap();
    let pair_received_bson = bson::from_slice::<Bson>(&buffer).unwrap();

    debug!(
        "Socket sent data: {}, received data: {}",
        message, pair_received_bson
    );

    assert_eq!(pair_received_bson, message);

    // One time response. See logs if removed response from call registry
    let call = sender.call(&message).await;

    // Start loop to start receiving responses
    sender.start_loop().await;

    let mut call_response = call.recv().await.unwrap();

    let mut stream_received = call_response.take_fd().unwrap();
    debug!("Call response: {}", response_message.body());

    let mut buffer = BytesMut::new();
    stream_received.read_buf(&mut buffer).await.unwrap();
    let pair_received_bson = bson::from_slice::<Bson>(&buffer).unwrap();

    debug!(
        "Call socket sent data: {}, received data: {}",
        message, pair_received_bson
    );

    assert_eq!(pair_received_bson, message);
}
