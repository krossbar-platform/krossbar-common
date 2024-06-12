use std::{
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use futures::Future;

use crate::state::State;

/// State machine contrcutor
pub struct Machine<St, Err>
where
    St: Send,
    Err: Send,
{
    state: Option<St>,
    _phantom: PhantomData<Err>,
}

impl<St, Err> Machine<St, Err>
where
    St: Send + 'static,
    Err: Send + 'static,
{
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
        Fut: Future<Output = Result<Ret, Err>> + Send + 'static,
    {
        State::chain(Box::pin(self), func)
    }
}

impl<St, Err> Future for Machine<St, Err>
where
    St: Send,
    Err: Send,
{
    type Output = Result<St, Err>;

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        println!("YOBA - 3");
        Poll::Ready(Ok(self.state.take().unwrap()))
    }
}

impl<St, Err> Unpin for Machine<St, Err>
where
    St: Send,
    Err: Send,
{
}
