use futures::{select, FutureExt};
use karo_common_rpc::rpc::Rpc;
use tokio::net::UnixStream;

const ENDPOINT_NAME: &str = "test_function";

#[tokio::test]
async fn test_simple_call() {
    let (stream1, stream2) = UnixStream::pair().unwrap();

    let mut rpc1 = Rpc::new(stream1);
    let mut rpc2 = Rpc::new(stream2);

    let call = rpc1.call::<u32, u32>(ENDPOINT_NAME, &42).await.unwrap();

    // Poll the stream to receive the request
    let mut request = rpc2.poll().await.unwrap();
    assert_eq!(request.endpoint(), ENDPOINT_NAME);
    assert!(request.stream().is_none());

    let request_body: u32 = bson::from_bson(request.params().clone()).unwrap();
    assert_eq!(request_body, 42);

    assert!(request.respond(Ok(420)).await);

    select! {
        response = call.fuse() => {
            assert_eq!(response.unwrap(), 420);
        },
        _ = rpc1.poll().fuse() => {}
    }
}

#[tokio::test]
async fn test_call_reconnect() {
    let (stream1, stream2) = UnixStream::pair().unwrap();

    let mut rpc1 = Rpc::new(stream1);
    {
        let mut rpc2 = Rpc::new(stream2);

        let call = rpc1.call::<u32, u32>(ENDPOINT_NAME, &42).await.unwrap();

        // Poll the stream to receive the request
        let mut request = rpc2.poll().await.unwrap();
        assert_eq!(request.endpoint(), ENDPOINT_NAME);
        assert!(request.stream().is_none());

        let request_body: u32 = bson::from_bson(request.params().clone()).unwrap();
        assert_eq!(request_body, 42);

        assert!(request.respond(Ok(420)).await);

        select! {
            response = call.fuse() => {
                assert_eq!(response.unwrap(), 420);
            },
            _ = rpc1.poll().fuse() => {}
        }
    }

    let (stream1, stream3) = UnixStream::pair().unwrap();

    {
        rpc1.replace(Rpc::new(stream1)).await;
        let mut rpc3 = Rpc::new(stream3);

        let call = rpc1.call::<u32, u32>(ENDPOINT_NAME, &42).await.unwrap();

        // Poll the stream to receive the request
        let mut request = rpc3.poll().await.unwrap();
        assert_eq!(request.endpoint(), ENDPOINT_NAME);
        assert!(request.stream().is_none());

        let request_body: u32 = bson::from_bson(request.params().clone()).unwrap();
        assert_eq!(request_body, 42);

        assert!(request.respond(Ok(420)).await);

        select! {
            response = call.fuse() => {
                assert_eq!(response.unwrap(), 420);
            },
            _ = rpc1.poll().fuse() => {}
        }
    }
}
