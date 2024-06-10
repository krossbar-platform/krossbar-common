use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::Future;

use crate::{control::Control, machine::Machine};

pub struct Init<State> {
    value: Option<State>,
}

impl<State: 'static> Init<State> {
    pub fn then<NRet, NFut>(self, func: fn(State) -> NFut) -> Machine<State, NRet, NFut>
    where
        NRet: 'static,
        NFut: Future<Output = Control<State, NRet>> + 'static,
    {
        Machine::chain(Box::pin(self), func)
    }
}

impl<State> Future for Init<State> {
    type Output = State;

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.value.take().unwrap())
    }
}

impl<State> Unpin for Init<State> {}
