use futures::{select, FutureExt};
use karo_common_rpc::{request::Body, rpc::Rpc};
use tokio::{io::AsyncWriteExt, net::UnixStream};

const ENDPOINT_NAME: &str = "test_function";

#[tokio::test]
async fn test_simple_call() {
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

#[tokio::test]
async fn test_call_reconnect() {
    let (stream1, stream2) = UnixStream::pair().unwrap();

    let mut rpc1 = Rpc::new(stream1);
    {
        let mut rpc2 = Rpc::new(stream2);

        let call = rpc1.call::<u32, u32>(ENDPOINT_NAME, &42).await.unwrap();

        // Poll the stream to receive the request
        let mut request = rpc2.poll().await.unwrap();

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

    let (stream1, stream3) = UnixStream::pair().unwrap();

    {
        rpc1.replace(Rpc::new(stream1)).await;
        let mut rpc3 = Rpc::new(stream3);

        let call = rpc1.call::<u32, u32>(ENDPOINT_NAME, &42).await.unwrap();

        // Poll the stream to receive the request
        let mut request = rpc3.poll().await.unwrap();
        assert_eq!(request.endpoint(), ENDPOINT_NAME);

        if let Some(Body::Call(bson)) = request.take_body() {
            let request_body: u32 = bson::from_bson(bson).unwrap();
            assert_eq!(request_body, 42);
        } else {
            assert!(false, "Invalid message type")
        }

        assert!(request.respond(Ok(421)).await);

        select! {
            response = call.fuse() => {
                assert_eq!(response.unwrap(), 421);
            },
            _ = rpc1.poll().fuse() => {}
        }
    }
}

#[tokio::test]
async fn test_bson_param_error() {
    let (stream1, stream2) = UnixStream::pair().unwrap();

    let rpc1 = Rpc::new(stream1);
    let _ = Rpc::new(stream2);

    // Try to send u64::MAX, which BSON doesn't support. It has only i64
    let call = rpc1.call::<u64, u32>(ENDPOINT_NAME, &u64::MAX).await;

    assert!(matches!(
        call,
        Err(karo_common_rpc::Error::ParamsTypeError(_))
    ))
}

#[tokio::test]
async fn test_result_type_error() {
    let (stream1, stream2) = UnixStream::pair().unwrap();

    let mut rpc1 = Rpc::new(stream1);
    let mut rpc2 = Rpc::new(stream2);

    let call = rpc1.call::<u32, String>(ENDPOINT_NAME, &42).await.unwrap();

    // Poll the stream to receive the request
    let request = rpc2.poll().await.unwrap();

    assert!(request.respond(Ok(420)).await);

    select! {
        response = call.fuse() => {
            assert!(matches!(
                response,
                Err(karo_common_rpc::Error::ResultTypeError(_))
            ))
        },
        _ = rpc1.poll().fuse() => {}
    }
}

#[tokio::test]
async fn test_client_disconnected_error() {
    let (mut stream1, mut stream2) = UnixStream::pair().unwrap();

    stream1.shutdown().await.unwrap();
    stream2.shutdown().await.unwrap();
    let rpc1 = Rpc::new(stream1);

    // Try to send u64::MAX, which BSON doesn't support. It has only i64
    let call = rpc1.call::<u64, u32>(ENDPOINT_NAME, &42).await.unwrap();

    assert!(matches!(
        call.await,
        Err(karo_common_rpc::Error::PeerDisconnected)
    ))
}
