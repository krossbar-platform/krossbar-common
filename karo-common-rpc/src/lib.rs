/*!
RPC library used by Karo platform for communication.

The library:
- Receives [tokio::net::UnixStream] and returns RPC handle;
- Allows making calls, subscribing to an endpoint, and sending [tokio::net::UnixStream] using RPC connection;
- Supports replacing the stream after reconnection, resubscribing to the active subscriptions, and keeping all client handles valid;
- Supports message exchange monitoring via [Monitor]

Use [rpc::Rpc::poll] method to poll the stream. This includes waiting for a call or subscriptions response.

# Examples

RPC calls:
```
let stream = UnixStream::connect("/tmp/hub.sock").await.unwrap();
let mut rpc = Rpc::new(stream);

let call = rpc.call::<u32, u32>("echo", &42).await.unwrap();

select! {
    response = call.fuse() => {
        println!("Call response: {response}")
    },
    _ = rpc.poll().fuse() => {}
}
```

RPC subscription:
```
let stream = UnixStream::connect("/tmp/hub.sock").await.unwrap();
let mut rpc = Rpc::new(stream);

let subscription = subscribe::<u32>("subscription", &42).await.unwrap();

select! {
    response = subscription.take(2).collect::<Vec<Result<u32>>>() => {
        println!("Subscription response: {response}")
    },
    _ = rpc.poll().fuse() => {}
}
```

Polling imcoming messages:
```
let stream = UnixStream::connect("/tmp/hub.sock").await.unwrap();
let mut rpc = Rpc::new(stream);

while true {
    let request = rpc.poll().await {
        if request.is_none() {
            println!("Client disconnected");
            return;
        }
    }

    println!("Incoming method call: {}", request.endpoint());
    match request.take_body().unwrap() {
        Body::Call(bson) => {
            println!("Incoming call: {bson:?}");
            request.respond(Ok(bson))
        },
        Body::Subscription => {
            println!("Incoming subscription");
            request.respond(Ok(41));
            request.respond(Ok(42));
            request.respond(Ok(43));
        },
        Body::Fd(client_name, _) => {
            println!("Incoming connection request from {client_name}");
            request.respond(Ok(()))
        }
    }
}
```

See `tests/` for more examples.
*/

mod calls_registry;
mod error;
mod message;
mod message_stream;
#[cfg(feature = "monitor")]
pub mod monitor;
pub mod request;
pub mod rpc;
pub mod writer;

pub use error::*;

#[cfg(feature = "impl-monitor")]
pub use message::*;
#[cfg(feature = "impl-monitor")]
pub use monitor::*;
