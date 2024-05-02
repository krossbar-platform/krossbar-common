use futures::{select, FutureExt, StreamExt};
use karo_common_rpc::{request::Body, rpc::Rpc};
use tokio::net::UnixStream;

const ENDPOINT_NAME: &str = "test_function";

#[tokio::test]
async fn test_simple_subscription() {
    let (stream1, stream2) = UnixStream::pair().unwrap();

    let mut rpc1 = Rpc::new(stream1);
    let mut rpc2 = Rpc::new(stream2);

    let call = rpc1.subscribe::<u32>(ENDPOINT_NAME).await.unwrap();

    // Poll the stream to receive the request
    let mut request = rpc2.poll().await.unwrap();
    assert_eq!(request.endpoint(), ENDPOINT_NAME);

    assert!(matches!(request.take_body(), Some(Body::Subscription)));

    assert!(request.respond(Ok(420)).await);
    assert!(request.respond(Ok(421)).await);

    select! {
        response = call.take(2).collect::<Vec<karo_common_rpc::Result<u32>>>() => {
            assert!(matches!(response[0], Ok(420)));
            assert!(matches!(response[1], Ok(421)));
        },
        _ = rpc1.poll().fuse() => {}
    }
}

#[tokio::test]
async fn test_subscription_reconnect() {
    let (stream1, stream2) = UnixStream::pair().unwrap();

    let mut rpc1 = Rpc::new(stream1);
    {
        let mut rpc2 = Rpc::new(stream2);

        let call = rpc1.subscribe::<u32>(ENDPOINT_NAME).await.unwrap();

        // Poll the stream to receive the request
        let mut request = rpc2.poll().await.unwrap();
        assert_eq!(request.endpoint(), ENDPOINT_NAME);

        assert!(matches!(request.take_body(), Some(Body::Subscription)));

        assert!(request.respond(Ok(420)).await);
        assert!(request.respond(Ok(421)).await);

        select! {
            response = call.take(2).collect::<Vec<karo_common_rpc::Result<u32>>>() => {
                assert!(matches!(response[0], Ok(420)));
                assert!(matches!(response[1], Ok(421)));
            },
            _ = rpc1.poll().fuse() => {}
        }
    }

    let (stream1, stream3) = UnixStream::pair().unwrap();

    {
        rpc1.replace(Rpc::new(stream1)).await;
        let mut rpc3 = Rpc::new(stream3);

        let call = rpc1.subscribe::<u32>(ENDPOINT_NAME).await.unwrap();

        // Poll the stream to receive the request
        let mut request = rpc3.poll().await.unwrap();
        assert_eq!(request.endpoint(), ENDPOINT_NAME);

        assert!(matches!(request.take_body(), Some(Body::Subscription)));

        assert!(request.respond(Ok(420)).await);
        assert!(request.respond(Ok(421)).await);

        select! {
            response = call.take(2).collect::<Vec<karo_common_rpc::Result<u32>>>() => {
                assert!(matches!(response[0], Ok(420)));
                assert!(matches!(response[1], Ok(421)));
            },
            _ = rpc1.poll().fuse() => {}
        }
    }
}
