use super::handle::{Handle, HandleInner};
use super::task_waker::TaskWaker;
use super::Task;

use crate::io::driver::{self, Driver};
use crate::runtime::park::{ParkThread, UnparkThread};

use std::sync::{Arc, Mutex};
use std::task;

use crossbeam_queue::SegQueue;

pub(crate) struct Worker {
    handle: Handle,
    driver: Arc<Mutex<Driver>>,
    parker: ParkThread,
}

/// Share states for the workers.
#[derive(Default)]
pub(crate) struct Shared {
    pub(super) task: SegQueue<Task>,
    pub(super) unparker: SegQueue<UnparkThread>,
}

impl Worker {
    pub(crate) fn create(
        size: usize,
        driver: Driver,
        driver_handle: driver::Handle,
    ) -> (Vec<Worker>, Handle) {
        let driver = Arc::new(Mutex::new(driver));
        let shared = Shared::default();
        let handle = Handle(Arc::new(HandleInner {
            shared,
            driver: driver_handle,
        }));

        let workers = (0..size)
            .map(|_| Worker {
                handle: handle.clone(),
                driver: driver.clone(),
                parker: ParkThread::new(),
            })
            .collect();

        (workers, handle)
    }

    pub(crate) fn run(&self) {
        loop {
            let mut task = {
                loop {
                    if let Some(task) = self.handle.shared.task.pop() {
                        break task;
                    }

                    // No tasks.
                    //
                    // Poll events if acquire the lock,
                    // otherwise park the thread.
                    if let Ok(mut driver) = self.driver.try_lock() {
                        driver.poll_events();
                    } else {
                        self.handle.shared.unparker.push(self.parker.unpark());
                        self.parker.park();
                    }
                }
            };

            // Poll the task if there is one
            let inner = Arc::new(TaskWaker::new(self.handle.clone()));
            let waker = task::Waker::from(inner.clone());
            let mut cx = task::Context::from_waker(&waker);

            if task.as_mut().poll(&mut cx).is_pending() {
                // Put the task into the waker, then in the
                // next wakeup, the task will be rescheduled
                inner.task(task);
            }
        }
    }
}
