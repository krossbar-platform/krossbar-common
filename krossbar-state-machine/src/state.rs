use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{Future, FutureExt};

/// Machine state
pub struct State<St, Ret, Err, Fut>
where
    Fut: Future<Output = Result<Ret, Err>> + Send,
{
    /// Previos state
    fut: Pin<Box<dyn Future<Output = Result<St, Err>> + Send>>,
    /// Current state handler
    func: fn(St) -> Fut,
    /// State handle future
    func_fut: Option<Pin<Box<Fut>>>,
}

impl<St: 'static, Ret: 'static, Err: 'static, Fut> State<St, Ret, Err, Fut>
where
    Fut: Future<Output = Result<Ret, Err>> + Send + 'static,
{
    pub(crate) fn chain(
        fut: Pin<Box<dyn Future<Output = Result<St, Err>> + Send>>,
        func: fn(St) -> Fut,
    ) -> Self {
        Self {
            fut,
            func,
            func_fut: None,
        }
    }

    /// Add machine state
    pub fn then<NRet, NFut>(self, func: fn(Ret) -> NFut) -> State<Ret, NRet, Err, NFut>
    where
        NRet: 'static,
        NFut: Future<Output = Result<NRet, Err>> + Send + 'static,
    {
        State::chain(Box::pin(self), func)
    }

    /// Handle final state result
    pub fn unwrap<NRet>(
        self,
        func: fn(Result<Ret, Err>) -> NRet,
    ) -> impl Future<Output = NRet> + Send
    where
        NRet: 'static,
    {
        self.map(move |value| func(value))
    }
}

impl<St, Ret, Err, Fut> Future for State<St, Ret, Err, Fut>
where
    Fut: Future<Output = Result<Ret, Err>> + Send,
{
    type Output = Result<Ret, Err>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Check if we're already made a future from the function
        let mut func_fut = match self.func_fut.take() {
            // Already polled previous state, and created a future from the state function
            // Use it to act as a self future
            Some(fut) => fut,
            // Newly created state. Poll previous state first, and create a future from the
            // inner fuction to poll
            _ => {
                let state = match self.fut.as_mut().poll(cx) {
                    Poll::Ready(value) => match value {
                        Ok(state) => state,
                        Err(e) => return Poll::Ready(Err(e)),
                    },
                    Poll::Pending => return Poll::Pending,
                };

                Box::pin((self.func)(state))
            }
        };

        match func_fut.as_mut().poll(cx) {
            Poll::Ready(value) => Poll::Ready(value),
            Poll::Pending => {
                // Save function result future to pull later
                self.func_fut = Some(func_fut);
                Poll::Pending
            }
        }
    }
}

impl<St, Ret, Err, Fut> Unpin for State<St, Ret, Err, Fut> where
    Fut: Future<Output = Result<Ret, Err>> + Send
{
}
