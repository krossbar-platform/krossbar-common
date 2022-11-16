use bson::bson;
use tempdir::TempDir;

mod implementations;

use implementations::{
    simple_echo_listener::SimpleEchoListener, simple_echo_sender::SimpleEchoSender,
};

#[tokio::test(flavor = "multi_thread")]
async fn test_reconnect() {
    let socket_dir = TempDir::new("karo_hub_socket_dir").expect("Failed to create socket tempdir");
    let socket_path: String = socket_dir
        .path()
        .join("karo_hub.socket")
        .as_os_str()
        .to_str()
        .unwrap()
        .into();

    let _ = SimpleEchoListener::new(&socket_path).await;
    let mut connection = SimpleEchoSender::new(&socket_path).await;

    let bson = bson!({
        "message": "Hello world!"
    });

    let response = connection.send_receive(&bson).await;
    assert_eq!(response, bson);
}
