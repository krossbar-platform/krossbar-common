use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::Future;

use crate::{control::Control, stage::Stage};

pub struct Machine<State> {
    state: Option<State>,
}

impl<State: 'static> Machine<State> {
    pub fn init(state: State) -> Self {
        Self { state: Some(state) }
    }

    pub fn then<NRet, NFut>(self, func: fn(State) -> NFut) -> Stage<State, NRet, NFut>
    where
        NRet: 'static,
        NFut: Future<Output = Control<State, NRet>> + 'static,
    {
        Stage::chain(Box::pin(self), func)
    }
}

impl<State> Future for Machine<State> {
    type Output = State;

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.state.take().unwrap())
    }
}

impl<State> Unpin for Machine<State> {}
