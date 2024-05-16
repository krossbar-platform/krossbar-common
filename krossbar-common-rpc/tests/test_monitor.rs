use core::panic;

use futures::{lock::Mutex, select, FutureExt, StreamExt};
use once_cell::sync::Lazy;
use tokio::net::UnixStream;

use krossbar_common_rpc::{
    monitor::Monitor, request::Body, rpc::Rpc, Direction, MonitorMessage, RpcData,
};

const CLIENT_NAME: &str = "com.test.client";
const ENDPOINT_NAME: &str = "test_function";

// Need to execute test sequentially, because monitor messages will be mixed
static MONITOR_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

async fn next_monitor_message(rpc: &mut Rpc) -> MonitorMessage {
    let mut request = rpc.poll().await.unwrap();

    match request.take_body().unwrap() {
        Body::Message(body) => bson::from_bson::<MonitorMessage>(body).unwrap(),
        _ => panic!("Invalid monitor message type"),
    }
}

#[tokio::test]
async fn test_monitor_fd_send() {
    let _guard = MONITOR_LOCK.lock().await;

    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let (monitor_send, monitor_receive) = UnixStream::pair().unwrap();

    Monitor::set(monitor_send).await;

    let (stream1, stream2) = UnixStream::pair().unwrap();

    let rpc1 = Rpc::new(stream1);
    let mut rpc2 = Rpc::new(stream2);

    let (_send_stream1, send_stream2) = UnixStream::pair().unwrap();

    rpc1.connection_request("rpc1", CLIENT_NAME, send_stream2)
        .await
        .unwrap();

    // Poll the stream to receive the request
    let request = rpc2.poll().await.unwrap();
    assert_eq!(request.endpoint(), "connect");

    let mut monitor_receiver = Rpc::new(monitor_receive);

    let sent_message = next_monitor_message(&mut monitor_receiver).await;
    assert!(matches!(sent_message.direction, Direction::Ougoing));
    assert!(matches!(
        sent_message.message.data,
        RpcData::ConnectionRequest { .. }
    ));

    let received_message = next_monitor_message(&mut monitor_receiver).await;
    assert!(matches!(received_message.direction, Direction::Incoming));
    assert!(matches!(
        received_message.message.data,
        RpcData::ConnectionRequest { .. }
    ));
}

#[tokio::test]
async fn test_monitor_fd_response() {
    let _guard = MONITOR_LOCK.lock().await;

    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let (monitor_send, monitor_receive) = UnixStream::pair().unwrap();

    Monitor::set(monitor_send).await;

    let (stream1, stream2) = UnixStream::pair().unwrap();

    let mut rpc1 = Rpc::new(stream1);
    let mut rpc2 = Rpc::new(stream2);

    let call = rpc1.call_fd::<u32, u32>(ENDPOINT_NAME, &42).await.unwrap();

    // Poll the stream to receive the request
    let request = rpc2.poll().await.unwrap();

    // Respond
    let (_send_stream1, send_stream2) = UnixStream::pair().unwrap();
    assert!(request.respond_with_fd(Ok(420), send_stream2).await);

    // Poll the stream to receive the request
    let _ = select! {
        response = call.fuse() => {
            let (data, stream) = response.unwrap();
            assert_eq!(data, 420);
            stream
        },
        _ = rpc1.poll().fuse() => {
            panic!("Should not return here")
        }
    };

    let mut monitor_receiver = Rpc::new(monitor_receive);

    // FD request
    let sent_message = next_monitor_message(&mut monitor_receiver).await;
    assert!(matches!(sent_message.direction, Direction::Ougoing));
    assert!(matches!(sent_message.message.data, RpcData::Call { .. }));

    // FD request reseived
    let received_message = next_monitor_message(&mut monitor_receiver).await;
    assert!(matches!(received_message.direction, Direction::Incoming));
    assert!(matches!(
        received_message.message.data,
        RpcData::Call { .. }
    ));

    // FD response
    let sent_fd_message = next_monitor_message(&mut monitor_receiver).await;
    assert!(matches!(sent_fd_message.direction, Direction::Ougoing));
    assert!(matches!(
        sent_fd_message.message.data,
        RpcData::FdResponse(_)
    ));

    // FD response received
    let received_fd_message = next_monitor_message(&mut monitor_receiver).await;
    assert!(matches!(received_fd_message.direction, Direction::Incoming));
    assert!(matches!(
        received_fd_message.message.data,
        RpcData::FdResponse(_)
    ));
}

#[tokio::test]
async fn test_monitor_subscription() {
    let _guard = MONITOR_LOCK.lock().await;

    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let (monitor_send, monitor_receive) = UnixStream::pair().unwrap();

    Monitor::set(monitor_send).await;

    let (stream1, stream2) = UnixStream::pair().unwrap();

    let mut rpc1 = Rpc::new(stream1);
    let mut rpc2 = Rpc::new(stream2);

    let mut subscription = rpc1.subscribe::<u32>(ENDPOINT_NAME).await.unwrap();

    // Send responses
    let request = rpc2.poll().await.unwrap();
    assert!(request.respond(Ok(420)).await);
    assert!(request.respond(Ok(421)).await);

    // Receive responses
    select! {
        _ = subscription.next() => {},
        _ = rpc1.poll().fuse() => {}
    };

    select! {
        _ = subscription.next() => {},
        _ = rpc1.poll().fuse() => {}
    };

    let mut monitor_receiver = Rpc::new(monitor_receive);

    let sent_subscription_request = next_monitor_message(&mut monitor_receiver).await;
    assert!(matches!(
        sent_subscription_request.direction,
        Direction::Ougoing
    ));
    assert!(matches!(
        sent_subscription_request.message.data,
        RpcData::Subscription { .. }
    ));

    let received_subscription_request = next_monitor_message(&mut monitor_receiver).await;
    assert!(matches!(
        received_subscription_request.direction,
        Direction::Incoming
    ));
    assert!(matches!(
        received_subscription_request.message.data,
        RpcData::Subscription { .. }
    ));

    // Outgoing subscription messages
    let send_subscription_message1 = next_monitor_message(&mut monitor_receiver).await;
    assert!(matches!(
        send_subscription_message1.direction,
        Direction::Ougoing
    ));
    assert!(matches!(
        send_subscription_message1.message.data,
        RpcData::Response(_)
    ));

    let send_subscription_message2 = next_monitor_message(&mut monitor_receiver).await;
    assert!(matches!(
        send_subscription_message2.direction,
        Direction::Ougoing
    ));
    assert!(matches!(
        send_subscription_message2.message.data,
        RpcData::Response(_)
    ));

    // Incoming subscription messages
    let received_subscription_message1 = next_monitor_message(&mut monitor_receiver).await;
    assert!(matches!(
        received_subscription_message1.direction,
        Direction::Incoming
    ));
    assert!(matches!(
        received_subscription_message1.message.data,
        RpcData::Response(_)
    ));

    let received_subscription_message1 = next_monitor_message(&mut monitor_receiver).await;
    assert!(matches!(
        received_subscription_message1.direction,
        Direction::Incoming
    ));
    assert!(matches!(
        received_subscription_message1.message.data,
        RpcData::Response(_)
    ));
}
