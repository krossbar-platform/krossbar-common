use std::{
    pin::{pin, Pin},
    task::{Context, Poll},
};

use futures::{Future, FutureExt};

/// Machine state
pub struct State<St, Ret, Err, Fut>
where
    Fut: Future<Output = Result<Ret, Err>>,
{
    fut: Pin<Box<dyn Future<Output = Result<St, Err>> + Send>>,
    func: fn(St) -> Fut,
}

impl<St: 'static, Ret: 'static, Err: 'static, Fut> State<St, Ret, Err, Fut>
where
    Fut: Future<Output = Result<Ret, Err>> + 'static,
{
    pub(crate) fn chain(
        fut: Pin<Box<dyn Future<Output = Result<St, Err>> + Send>>,
        func: fn(St) -> Fut,
    ) -> Self {
        Self { fut, func }
    }

    /// Add machine state
    pub fn then<NRet, NFut>(self, func: fn(Ret) -> NFut) -> State<Ret, NRet, Err, NFut>
    where
        NRet: 'static,
        NFut: Future<Output = Result<NRet, Err>> + 'static,
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
    Fut: Future<Output = Result<Ret, Err>>,
{
    type Output = Result<Ret, Err>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let state = match self.fut.as_mut().poll(cx) {
            Poll::Ready(value) => match value {
                Ok(state) => state,
                Err(e) => return Poll::Ready(Err(e)),
            },
            Poll::Pending => return Poll::Pending,
        };

        let mut future = pin!((self.func)(state));

        match future.as_mut().poll(cx) {
            Poll::Ready(value) => Poll::Ready(value),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<St, Ret, Err, Fut> Unpin for State<St, Ret, Err, Fut> where
    Fut: Future<Output = Result<Ret, Err>>
{
}
