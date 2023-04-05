use super::{Handle, Task};

use std::sync::{Arc, Mutex};
use std::task::Wake;

/// Waker for the top level future.
pub(crate) struct TaskWaker {
    handle: Handle,
    task: Mutex<Option<Task>>,
}

impl TaskWaker {
    pub(crate) fn new(handle: Handle) -> Self {
        Self {
            handle,
            task: Mutex::new(None),
        }
    }

    pub(crate) fn task(&self, task: Task) {
        self.task.lock().unwrap().replace(task);
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        // Reschedule the task at wakeup.
        if let Some(task) = self.task.lock().unwrap().take() {
            self.handle.schedule(task);
        }
    }
}
