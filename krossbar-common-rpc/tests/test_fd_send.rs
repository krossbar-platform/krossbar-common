use futures::{select, FutureExt};
use krossbar_common_rpc::{request::Body, rpc::Rpc};
use tokio::net::UnixStream;

const CLIENT_NAME: &str = "com.test.client";
const ENDPOINT_NAME: &str = "test_function";

async fn test_pair_call(mut rpc1: Rpc, mut rpc2: Rpc) {
    let call = rpc1.call::<u32, u32>(ENDPOINT_NAME, &42).await.unwrap();

    // Poll the stream to receive the request
    let mut request = rpc2.poll().await.unwrap();
    assert_eq!(request.endpoint(), ENDPOINT_NAME);

    if let Some(Body::Call(bson)) = request.take_body() {
        let request_body: u32 = bson::from_bson(bson).unwrap();
        assert_eq!(request_body, 42);
    } else {
        panic!("Invalid message type")
    }

    assert!(request.respond(Ok(420)).await);

    select! {
        response = call.fuse() => {
            assert_eq!(response.unwrap(), 420);
        },
        _ = rpc1.poll().fuse() => {}
    }
}

#[tokio::test]
async fn test_fd_send() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let (stream1, stream2) = UnixStream::pair().unwrap();

    let rpc1 = Rpc::new(stream1);
    let mut rpc2 = Rpc::new(stream2);

    let (send_stream1, send_stream2) = UnixStream::pair().unwrap();

    rpc1.connection_request(CLIENT_NAME, send_stream2)
        .await
        .unwrap();

    // Poll the stream to receive the request
    let mut request = rpc2.poll().await.unwrap();
    assert_eq!(request.endpoint(), "connect");

    let received_rpc = if let Some(Body::Fd(client_name, stream)) = request.take_body() {
        assert_eq!(client_name, CLIENT_NAME);
        Rpc::new(stream)
    } else {
        panic!("Invalid message type")
    };

    test_pair_call(received_rpc, Rpc::new(send_stream1)).await
}

#[tokio::test]
async fn test_no_fd_response() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let (stream1, stream2) = UnixStream::pair().unwrap();

    let mut rpc1 = Rpc::new(stream1);
    let mut rpc2 = Rpc::new(stream2);

    let call = rpc1.call_fd::<u32, u32>(ENDPOINT_NAME, &42).await.unwrap();

    // Poll the stream to receive the request
    let mut request = rpc2.poll().await.unwrap();
    assert_eq!(request.endpoint(), ENDPOINT_NAME);

    if let Some(Body::Call(bson)) = request.take_body() {
        let request_body: u32 = bson::from_bson(bson).unwrap();
        assert_eq!(request_body, 42);
    } else {
        panic!("Invalid message type")
    }

    assert!(
        request
            .respond::<u32>(Err(krossbar_common_rpc::Error::ClientError(
                "Test error".to_owned()
            )))
            .await
    );

    select! {
        response = call.fuse() => {
            assert!(matches!(response, Err(krossbar_common_rpc::Error::ClientError(_))));
        },
        _ = rpc1.poll().fuse() => {}
    }
}

#[tokio::test]
async fn test_fd_response() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let (stream1, stream2) = UnixStream::pair().unwrap();

    let mut rpc1 = Rpc::new(stream1);
    let mut rpc2 = Rpc::new(stream2);

    let call = rpc1.call_fd::<u32, u32>(ENDPOINT_NAME, &42).await.unwrap();

    // Poll the stream to receive the request
    let mut request = rpc2.poll().await.unwrap();
    assert_eq!(request.endpoint(), ENDPOINT_NAME);

    if let Some(Body::Call(bson)) = request.take_body() {
        let request_body: u32 = bson::from_bson(bson).unwrap();
        assert_eq!(request_body, 42);
    } else {
        panic!("Invalid message type")
    }

    let (send_stream1, send_stream2) = UnixStream::pair().unwrap();
    assert!(request.respond_with_fd(Ok(420), send_stream2).await);

    let received_stream = select! {
        response = call.fuse() => {
            let (data, stream) = response.unwrap();
            assert_eq!(data, 420);
            stream
        },
        _ = rpc1.poll().fuse() => {
            panic!("Should not return here")
        }
    };

    test_pair_call(Rpc::new(received_stream), Rpc::new(send_stream1)).await
}
