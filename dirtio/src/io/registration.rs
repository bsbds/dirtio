use crate::runtime::context;
use crate::runtime::scheduler::handle::Handle;

use std::io;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::task::{ready, Context, Poll};

use futures::channel::mpsc::UnboundedReceiver;
use futures::{Future, StreamExt};
use mio::event::{Event, Source};
use mio::{Interest, Token};

pub(crate) struct IoRegistration<S>
where
    S: Source,
{
    io: Option<S>,
    registration: Registration,
}

impl<S> IoRegistration<S>
where
    S: Source,
{
    pub fn new(mut io: S) -> io::Result<Self> {
        let registration = Registration::new(&mut io, Interest::READABLE | Interest::WRITABLE)?;
        Ok(Self {
            io: Some(io),
            registration,
        })
    }

    pub fn registration(&self) -> &Registration {
        &self.registration
    }
}

impl<S> Deref for IoRegistration<S>
where
    S: Source,
{
    type Target = S;

    fn deref(&self) -> &Self::Target {
        self.io.as_ref().unwrap()
    }
}

impl<S> Drop for IoRegistration<S>
where
    S: Source,
{
    fn drop(&mut self) {
        let _ = self.registration.deregister(self.io.as_mut().unwrap());
    }
}

// Read and write readiness for current registration.
struct Ready {
    read: AtomicBool,
    write: AtomicBool,
}

impl Ready {
    fn new() -> Self {
        Self {
            read: AtomicBool::new(false),
            write: AtomicBool::new(false),
        }
    }

    /// Clear readiness when WouldBlock.
    fn clear(&self, interest: Interest) {
        self.read
            .fetch_and(!interest.is_readable(), Ordering::SeqCst);
        self.write
            .fetch_and(!interest.is_writable(), Ordering::SeqCst);
    }

    /// Determines if current readiness satisfies the interest.
    fn ready(&self, interest: Interest) -> bool {
        (!interest.is_readable() || self.read.load(Ordering::Relaxed))
            && (!interest.is_writable() || self.write.load(Ordering::Relaxed))
    }

    /// Set current readiness with the event notified.
    fn set(&self, event: &Event) {
        self.read.fetch_or(event.is_readable(), Ordering::SeqCst);
        self.write.fetch_or(event.is_writable(), Ordering::SeqCst);
    }
}

pub(crate) struct Registration {
    token: Token,
    event_receiver: Mutex<UnboundedReceiver<Event>>,
    ready: Ready,
    handle: Handle,
}

impl Registration {
    pub(crate) fn new(io: &mut impl Source, interests: Interest) -> io::Result<Self> {
        let handle = context::current();
        let (token, event_receiver) = handle.driver.add_source(io, interests)?;
        Ok(Self {
            token,
            event_receiver: Mutex::new(event_receiver),
            ready: Ready::new(),
            handle,
        })
    }

    pub(crate) async fn async_io<R>(
        &self,
        interest: Interest,
        mut f: impl FnMut() -> io::Result<R>,
    ) -> io::Result<R> {
        loop {
            self.async_readiness(interest).await;

            match f() {
                // If the result is a `WouldBlock`, clear the readiness
                // for the specific interest.
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    self.clear_readiness(interest);
                }
                x => return x,
            }
        }
    }

    pub(crate) fn poll_io<R>(
        &self,
        cx: &mut Context<'_>,
        interest: Interest,
        mut f: impl FnMut() -> io::Result<R>,
    ) -> Poll<io::Result<R>> {
        loop {
            ready!(self.readiness(interest).poll_inner(cx));

            match f() {
                // If the result is a `WouldBlock`, clear the readiness
                // for the specific interest.
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    self.clear_readiness(interest);
                }
                x => return Poll::Ready(x),
            }
        }
    }

    pub(crate) fn deregister(&self, source: &mut impl Source) -> io::Result<()> {
        self.handle.driver.deregister_source(source, self.token)
    }

    async fn async_readiness(&self, interest: Interest) {
        self.readiness(interest).await;
    }

    fn readiness(&self, interest: Interest) -> Readiness {
        Readiness {
            registration: self,
            interest,
        }
    }

    fn clear_readiness(&self, interest: Interest) {
        self.ready.clear(interest);
    }

    fn poll_event(&self, cx: &mut Context<'_>) -> Poll<Event> {
        let mut event_receiver = self.event_receiver.lock().unwrap();
        Poll::Ready(ready!(event_receiver.poll_next_unpin(cx)).expect("channel disconnected"))
    }
}

/// Poll events and set readiness for current registration.
struct Readiness<'a> {
    registration: &'a Registration,
    interest: Interest,
}

impl<'a> Readiness<'a> {
    fn poll_inner(&self, cx: &mut Context<'_>) -> Poll<()> {
        loop {
            // Check readiness first.
            if self.registration.ready.ready(self.interest) {
                return Poll::Ready(());
            }
            // If readiness not satisfied, get an event from the driver
            // and check again.
            let event = ready!(self.registration.poll_event(cx));
            self.registration.ready.set(&event);
        }
    }
}

impl<'a> Future for Readiness<'a> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.poll_inner(cx)
    }
}
