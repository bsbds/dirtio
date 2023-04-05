use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{channel::oneshot, Future, FutureExt};

/// Poll the output of a spawned future
pub struct JoinHandle<R>(oneshot::Receiver<R>);

impl<R> JoinHandle<R> {
    pub fn new() -> (oneshot::Sender<R>, Self) {
        let (sender, receiver) = oneshot::channel();
        (sender, Self(receiver))
    }
}

impl<R> Future for JoinHandle<R> {
    type Output = R;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.poll_unpin(cx).map(Result::unwrap)
    }
}
