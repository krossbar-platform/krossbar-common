use std::{
    pin::{pin, Pin},
    task::{Context, Poll},
};

use futures::{Future, FutureExt};

use crate::control::Control;

pub struct Stage<State, Ret, Fut>
where
    Fut: Future<Output = Control<State, Ret>>,
{
    fut: Pin<Box<dyn Future<Output = State>>>,
    func: fn(State) -> Fut,
}

impl<State: 'static, Ret: 'static, Fut> Stage<State, Ret, Fut>
where
    Fut: Future<Output = Control<State, Ret>> + 'static,
{
    pub(crate) fn chain(fut: Pin<Box<dyn Future<Output = State>>>, func: fn(State) -> Fut) -> Self {
        Self { fut, func }
    }

    pub fn then<NRet, NFut>(self, func: fn(Ret) -> NFut) -> Stage<Ret, NRet, NFut>
    where
        NRet: 'static,
        NFut: Future<Output = Control<Ret, NRet>> + 'static,
    {
        Stage::chain(Box::pin(self), func)
    }

    pub fn ret<NRet>(self, func: fn(Ret) -> NRet) -> impl Future<Output = NRet>
    where
        NRet: 'static,
    {
        self.map(move |value| func(value))
    }
}

impl<State, Ret, Fut> Future for Stage<State, Ret, Fut>
where
    Fut: Future<Output = Control<State, Ret>>,
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

impl<State, Ret, Fut> Unpin for Stage<State, Ret, Fut> where
    Fut: Future<Output = Control<State, Ret>>
{
}
