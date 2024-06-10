use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::Future;

use crate::{control::Control, state::State};

/// State machine contrcutor
pub struct Machine<St> {
    state: Option<St>,
}

impl<St: 'static> Machine<St> {
    /// Init machine with a **state**
    pub fn init(state: St) -> Self {
        Self { state: Some(state) }
    }

    /// Add machine state
    pub fn then<NRet, NFut>(self, func: fn(St) -> NFut) -> State<St, NRet, NFut>
    where
        NRet: 'static,
        NFut: Future<Output = Control<St, NRet>> + 'static,
    {
        State::chain(Box::pin(self), func)
    }
}

impl<St> Future for Machine<St> {
    type Output = St;

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.state.take().unwrap())
    }
}

impl<St> Unpin for Machine<St> {}
