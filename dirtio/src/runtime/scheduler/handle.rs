use super::join_handle::JoinHandle;
use super::worker::Shared;
use super::Task;

use crate::io::driver;
use crate::runtime::context;

use std::sync::Arc;

use futures::{Future, FutureExt};

#[derive(Clone)]
pub(crate) struct Handle(pub(super) Arc<HandleInner>);

pub(crate) struct HandleInner {
    pub(super) shared: Shared,
    pub(crate) driver: driver::Handle,
}

impl Handle {
    pub(crate) fn enter(&self) {
        context::set_current(self.clone());
    }

    pub(crate) fn current() -> Self {
        context::current()
    }
}

impl std::ops::Deref for Handle {
    type Target = Arc<HandleInner>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl HandleInner {
    pub(crate) fn spawn<F>(&self, fut: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send,
    {
        let (sender, handle) = JoinHandle::new();
        let task = Box::pin(fut.map(|output| sender.send(output).unwrap_or_default()));
        self.schedule(task);
        handle
    }

    /// Schedule a task and wake up a worker to process it.
    pub(crate) fn schedule(&self, task: Task) {
        self.shared.task.push(task);
        if let Some(unparker) = self.shared.unparker.pop() {
            unparker.unpark();
        }
    }
}
