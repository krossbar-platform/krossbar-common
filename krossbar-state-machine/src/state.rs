use std::{
    pin::{pin, Pin},
    task::{Context, Poll},
};

use futures::{Future, FutureExt};

use crate::control::Control;

/// Machine state
pub struct State<St, Ret, Fut>
where
    Fut: Future<Output = Control<St, Ret>>,
{
    fut: Pin<Box<dyn Future<Output = St>>>,
    func: fn(St) -> Fut,
}

impl<St: 'static, Ret: 'static, Fut> State<St, Ret, Fut>
where
    Fut: Future<Output = Control<St, Ret>> + 'static,
{
    pub(crate) fn chain(fut: Pin<Box<dyn Future<Output = St>>>, func: fn(St) -> Fut) -> Self {
        Self { fut, func }
    }

    /// Add machine state
    pub fn then<NRet, NFut>(self, func: fn(Ret) -> NFut) -> State<Ret, NRet, NFut>
    where
        NRet: 'static,
        NFut: Future<Output = Control<Ret, NRet>> + 'static,
    {
        State::chain(Box::pin(self), func)
    }

    /// Handle final state result
    pub fn ret<NRet>(self, func: fn(Ret) -> NRet) -> impl Future<Output = NRet>
    where
        NRet: 'static,
    {
        self.map(move |value| func(value))
    }
}

impl<St, Ret, Fut> Future for State<St, Ret, Fut>
where
    Fut: Future<Output = Control<St, Ret>>,
{
    type Output = Ret;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = match self.fut.as_mut().poll(cx) {
            Poll::Ready(value) => value,
            Poll::Pending => return Poll::Pending,
        };

        loop {
            let mut future = pin!((self.func)(state));

            let control = match future.as_mut().poll(cx) {
                Poll::Ready(value) => value,
                Poll::Pending => return Poll::Pending,
            };

            state = match control {
                Control::Loop(state) => state,
                Control::Return(ret) => {
                    return Poll::Ready(ret);
                }
            };
        }
    }
}

impl<St, Ret, Fut> Unpin for State<St, Ret, Fut> where Fut: Future<Output = Control<St, Ret>> {}
