use futures::{select, StreamExt};
use karo_common_rpc::{request::Body, rpc::Rpc};
use tokio::net::UnixStream;

const ENDPOINT_NAME: &str = "test_function";

#[tokio::test]
async fn test_simple_subscription() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let (stream1, stream2) = UnixStream::pair().unwrap();

    let mut rpc1 = Rpc::new(stream1);
    let mut rpc2 = Rpc::new(stream2);

    let call = rpc1.subscribe::<u32>(ENDPOINT_NAME).await.unwrap();

    // Poll the stream to receive the request
    let mut request = rpc2.next().await.unwrap();
    assert_eq!(request.endpoint(), ENDPOINT_NAME);

    assert!(matches!(request.take_body(), Some(Body::Subscription)));

    assert!(request.respond(Ok(420)).await);
    assert!(request.respond(Ok(421)).await);

    select! {
        response = call.take(2).collect::<Vec<karo_common_rpc::Result<u32>>>() => {
            assert!(matches!(response[0], Ok(420)));
            assert!(matches!(response[1], Ok(421)));
        },
        _ = rpc1.next() => {}
    }
}

#[tokio::test]
async fn test_subscription_reconnect() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let (stream1, stream2) = UnixStream::pair().unwrap();

    let mut rpc1 = Rpc::new(stream1);
    let subscription1 = {
        let mut rpc2 = Rpc::new(stream2);

        let mut subscription = rpc1.subscribe::<u32>(ENDPOINT_NAME).await.unwrap();

        // Poll the stream to receive the request
        let mut request = rpc2.next().await.unwrap();
        assert_eq!(request.endpoint(), ENDPOINT_NAME);

        assert!(matches!(request.take_body(), Some(Body::Subscription)));

        assert!(request.respond(Ok(420)).await);
        assert!(request.respond(Ok(421)).await);

        select! {
            response = subscription.next() => {
                assert!(matches!(response.unwrap(), Ok(420)));
            },
            _ = rpc1.next() => {}
        };

        select! {
            response = subscription.next() => {
                assert!(matches!(response.unwrap(), Ok(421)));
            },
            _ = rpc1.next() => {}
        };

        subscription
    };

    let (stream1, stream3) = UnixStream::pair().unwrap();

    {
        rpc1.on_reconnected(Rpc::new(stream1)).await;
        let mut rpc3 = Rpc::new(stream3);

        // Get reconnection subscription request
        let mut request = rpc3.next().await.unwrap();
        assert_eq!(request.endpoint(), ENDPOINT_NAME);

        assert!(matches!(request.take_body(), Some(Body::Subscription)));

        assert!(request.respond(Ok(420)).await);
        assert!(request.respond(Ok(421)).await);

        select! {
            response = subscription1.take(2).collect::<Vec<karo_common_rpc::Result<u32>>>() => {
                assert!(matches!(response[0], Ok(420)));
                assert!(matches!(response[1], Ok(421)));
            },
            _ = rpc1.next() => {}
        }
    }
}

#[tokio::test]
async fn test_subscription_reconnect_new_request() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let (stream1, stream2) = UnixStream::pair().unwrap();

    let mut rpc1 = Rpc::new(stream1);
    {
        let mut rpc2 = Rpc::new(stream2);

        let mut subscription = rpc1.subscribe::<u32>(ENDPOINT_NAME).await.unwrap();

        // Poll the stream to receive the request
        let mut request = rpc2.next().await.unwrap();
        assert_eq!(request.endpoint(), ENDPOINT_NAME);

        assert!(matches!(request.take_body(), Some(Body::Subscription)));

        assert!(request.respond(Ok(420)).await);
        assert!(request.respond(Ok(421)).await);

        select! {
            response = subscription.next() => {
                assert!(matches!(response.unwrap(), Ok(420)));
            },
            _ = rpc1.next() => {}
        };

        select! {
            response = subscription.next() => {
                assert!(matches!(response.unwrap(), Ok(421)));
            },
            _ = rpc1.next() => {}
        };
    }

    let (stream1, stream3) = UnixStream::pair().unwrap();

    {
        rpc1.on_reconnected(Rpc::new(stream1)).await;
        let mut rpc3 = Rpc::new(stream3);

        let subscription2 = rpc1.subscribe::<u32>(ENDPOINT_NAME).await.unwrap();

        // This is a resubscription request
        let _ = rpc3.next().await.unwrap();

        // This is a new subscription request
        let mut sub2_request = rpc3.next().await.unwrap();
        assert_eq!(sub2_request.endpoint(), ENDPOINT_NAME);

        assert!(matches!(sub2_request.take_body(), Some(Body::Subscription)));

        assert!(sub2_request.respond(Ok(420)).await);
        assert!(sub2_request.respond(Ok(421)).await);

        select! {
            response = subscription2.take(2).collect::<Vec<karo_common_rpc::Result<u32>>>() => {
                assert!(matches!(response[0], Ok(420)));
                assert!(matches!(response[1], Ok(421)));
            },
            _ = rpc1.next() => {}
        }
    }
}
