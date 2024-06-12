use std::{
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use futures::Future;

use crate::state::State;

/// State machine contrcutor
pub struct Machine<St, Err> {
    state: Option<St>,
    _phantom: PhantomData<Err>,
}

impl<St: 'static, Err: 'static> Machine<St, Err> {
    /// Init machine with a **state**
    pub fn init(state: St) -> Self {
        Self {
            state: Some(state),
            _phantom: PhantomData,
        }
    }

    /// Add machine state
    pub fn then<Ret, Fut>(self, func: fn(St) -> Fut) -> State<St, Ret, Err, Fut>
    where
        Ret: 'static,
        Fut: Future<Output = Result<Ret, Err>> + 'static,
    {
        State::chain(Box::pin(self), func)
    }
}

impl<St, Err> Future for Machine<St, Err> {
    type Output = Result<St, Err>;

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(Ok(self.state.take().unwrap()))
    }
}

impl<St, Err> Unpin for Machine<St, Err> {}
