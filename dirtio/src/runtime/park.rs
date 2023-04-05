use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::task::{Wake, Waker};

const EMPTY: usize = 0;
const PARKED: usize = 1;
const NOTIFIED: usize = 2;

#[derive(Clone)]
pub(crate) struct ParkThread {
    inner: Arc<Inner>,
}

#[derive(Clone)]
pub(crate) struct UnparkThread {
    inner: Arc<Inner>,
}

struct Inner {
    state: AtomicUsize,
    mutex: Mutex<()>,
    condvar: Condvar,
}

impl ParkThread {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                state: AtomicUsize::new(EMPTY),
                mutex: Mutex::new(()),
                condvar: Condvar::new(),
            }),
        }
    }

    pub(crate) fn unpark(&self) -> UnparkThread {
        UnparkThread {
            inner: self.inner.clone(),
        }
    }

    pub(crate) fn park(&self) {
        self.inner.park();
    }
}

impl UnparkThread {
    pub(crate) fn unpark(&self) {
        self.inner.unpark();
    }

    pub(crate) fn into_waker(self) -> Waker {
        Waker::from(Arc::new(self))
    }
}

impl Wake for UnparkThread {
    fn wake(self: Arc<Self>) {
        self.unpark();
    }
}

impl Inner {
    fn park(&self) {
        // return immediately when previously notified
        if self
            .state
            .compare_exchange(NOTIFIED, EMPTY, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            return;
        }

        let mut m = self.mutex.lock().unwrap();

        match self
            .state
            .compare_exchange(EMPTY, PARKED, Ordering::SeqCst, Ordering::SeqCst)
        {
            Ok(_) => {}
            Err(NOTIFIED) => {
                // Unpark may called again after the `compare_exchange`
                // use swap instead of a store to perform an acquire operation
                // to observe writes to the state.
                let _ = self.state.swap(EMPTY, Ordering::SeqCst);
                return;
            }
            Err(actual) => panic!("inconsistent state in park, actual: {}", actual),
        }

        loop {
            m = self.condvar.wait(m).unwrap();
            if self
                .state
                .compare_exchange(NOTIFIED, EMPTY, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return;
            }
        }
    }

    fn unpark(&self) {
        // Use swap instead of comapre_exchange to make sure
        // there's always a release operation for park to
        // synchronize with.
        match self.state.swap(NOTIFIED, Ordering::SeqCst) {
            EMPTY => return,
            NOTIFIED => return,
            PARKED => {}
            _ => panic!("inconsistent state in unpark"),
        }

        // Drop immediately to avoid waiting the lock
        // when notified in `park`.
        drop(self.mutex.lock().unwrap());

        self.condvar.notify_one();
    }
}
