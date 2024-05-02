use futures::{select, FutureExt};
use karo_common_rpc::{request::Body, rpc::Rpc};
use tokio::net::UnixStream;

const ENDPOINT_NAME: &str = "test_function";

#[tokio::test]
async fn test_fd_send() {
    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let (stream1, stream2) = UnixStream::pair().unwrap();

    let mut rpc1 = Rpc::new(stream1);
    let mut rpc2 = Rpc::new(stream2);

    let call = rpc1.call::<u32, u32>(ENDPOINT_NAME, &42).await.unwrap();

    // Poll the stream to receive the request
    let mut request = rpc2.poll().await.unwrap();
    assert_eq!(request.endpoint(), ENDPOINT_NAME);

    if let Some(Body::Call(bson)) = request.take_body() {
        let request_body: u32 = bson::from_bson(bson).unwrap();
        assert_eq!(request_body, 42);
    } else {
        assert!(false, "Invalid message type")
    }

    assert!(request.respond(Ok(420)).await);

    select! {
        response = call.fuse() => {
            assert_eq!(response.unwrap(), 420);
        },
        _ = rpc1.poll().fuse() => {}
    }
}
