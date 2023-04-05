use super::park::ParkThread;
use super::scheduler::handle::Handle;
use super::scheduler::worker::Worker;

use crate::io::driver::Driver;

use std::io;
use std::task::{Context, Poll};
use std::thread;

use futures::{pin_mut, Future};

pub struct Runtime {
    handle: Handle,
}

impl Runtime {
    pub fn block_on<F>(&self, fut: F) -> F::Output
    where
        F: Future,
    {
        self.handle.enter();

        let park_thread = ParkThread::new();
        let unpark_thread = park_thread.unpark();

        let waker = unpark_thread.into_waker();
        let mut cx = Context::from_waker(&waker);

        pin_mut!(fut);

        loop {
            match fut.as_mut().poll(&mut cx) {
                Poll::Ready(r) => break r,
                Poll::Pending => {
                    park_thread.park();
                }
            }
        }
    }
}

#[derive(Default)]
pub struct Builder {
    worker_threads: Option<usize>,
}

impl Builder {
    pub fn new() -> Self {
        Self {
            worker_threads: None,
        }
    }

    pub fn worker_threads(&mut self, val: usize) -> &mut Self {
        self.worker_threads = Some(val);
        self
    }

    pub fn build(&mut self) -> io::Result<Runtime> {
        let (driver, driver_handle) = Driver::new()?;
        let worker_threads = self.worker_threads.unwrap_or_else(|| {
            thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get)
        });

        let (mut workers, handle) = Worker::create(worker_threads, driver, driver_handle);

        for worker in workers.drain(..) {
            let handle = handle.clone();
            let _ = thread::spawn(move || {
                handle.enter();
                worker.run();
            });
        }

        Ok(Runtime { handle })
    }
}
