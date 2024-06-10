use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::Future;

pub struct Init<State> {
    value: Option<State>,
}

impl<State> Unpin for Init<State> {}

impl<State> Future for Init<State> {
    type Output = State;

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.value.take().unwrap())
    }
}
