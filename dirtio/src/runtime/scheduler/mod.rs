pub(crate) mod handle;
pub(crate) mod join_handle;
mod task_waker;
pub(crate) mod worker;

use handle::Handle;
use join_handle::JoinHandle;

use std::pin::Pin;

use futures::Future;

/// A top level future.
pub type Task = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

pub fn spawn<F>(fut: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send,
{
    let handle = Handle::current();
    handle.spawn(fut)
}
