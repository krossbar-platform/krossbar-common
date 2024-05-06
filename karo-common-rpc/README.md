[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]

[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/karo-platform/karo-common/blob/main/LICENSE
[actions-badge]: https://github.com/karo-platform/karo-common/actions/workflows/rust.yml/badge.svg
[actions-url]: https://github.com/karo-platform/karo-common/actions/workflows/rust.yml

# karo-common-rpc

RPC library used by Karo platform for communication.

The library:
- Receives [tokio::net::UnixStream] and returns RPC handle;
- Allows making calls, subscribing to an endpoint, and sending [tokio::net::UnixStream] using RPC connection;
- Supports replacing the stream after reconnection, resubscribing to the active subscriptions, and keeping all client handles valid;
- Supports message exchange monitoring via [Monitor]

Use [rpc::Rpc::poll] method to poll the stream. This includes waiting for a call or subscriptions response.

## Examples

RPC calls:
```rust
use futures::{select, FutureExt};
use tokio::net::UnixStream;

use karo_common_rpc::rpc::Rpc;

async fn call() {
    let stream = UnixStream::connect("/tmp/hub.sock").await.unwrap();
    let mut rpc = Rpc::new(stream);

    let call = rpc.call::<u32, u32>("echo", &42).await.unwrap();

    select! {
        response = call.fuse() => {
            println!("Call response: {response:?}")
        },
        _ = rpc.poll().fuse() => {}
    }
}
```

RPC subscription:
```rust
use futures::{select, FutureExt, StreamExt};
use tokio::net::UnixStream;

use karo_common_rpc::rpc::Rpc;

async fn subscribe() {
    let stream = UnixStream::connect("/tmp/hub.sock").await.unwrap();
    let mut rpc = Rpc::new(stream);

    let subscription = rpc.subscribe::<u32>("signal").await.unwrap();

    select! {
        response = subscription.take(2).collect::<Vec<karo_common_rpc::Result<u32>>>() => {
            println!("Subscription response: {response:?}")
        },
        _ = rpc.poll().fuse() => {}
    }
}
```

Polling imcoming messages:
```rust
use futures::{select, FutureExt};
use tokio::net::UnixStream;

use karo_common_rpc::{rpc::Rpc, request::Body};

async fn poll() {
    let stream = UnixStream::connect("/tmp/hub.sock").await.unwrap();
    let mut rpc = Rpc::new(stream);

    loop {
        let request = rpc.poll().await;

        if request.is_none() {
            println!("Client disconnected");
            return;
        }

        let mut request = request.unwrap();

        println!("Incoming method call: {}", request.endpoint());
        match request.take_body().unwrap() {
            Body::Call(bson) => {
                println!("Incoming call: {bson:?}");
                request.respond(Ok(bson)).await;
            },
            Body::Subscription => {
                println!("Incoming subscription");
                request.respond(Ok(41)).await;
                request.respond(Ok(42)).await;
                request.respond(Ok(43)).await;
            },
            Body::Fd(client_name, _) => {
                println!("Incoming connection request from {client_name}");
                request.respond(Ok(())).await;
            }
        }
    }
}
```

See `tests/` for more examples.
