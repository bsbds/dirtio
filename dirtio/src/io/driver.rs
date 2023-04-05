use std::io;
use std::sync::Arc;
use std::time::Duration;

use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use mio::event::{Event, Source};
use mio::{Events, Interest, Poll, Registry, Token};
use sharded_slab::Slab;

pub(crate) struct Driver {
    poll: Poll,
    events: Events,
    wakers: Arc<Slab<UnboundedSender<Event>>>,
}

/// Handle to the IO driver.
pub(crate) struct Handle {
    registry: Registry,
    wakers: Arc<Slab<UnboundedSender<Event>>>,
}

impl Driver {
    pub(crate) fn new() -> io::Result<(Driver, Handle)> {
        let poll = Poll::new()?;
        let registry = poll.registry().try_clone()?;
        let wakers = Arc::new(Slab::new());

        let driver = Driver {
            poll,
            events: Events::with_capacity(128),
            wakers: wakers.clone(),
        };
        let handle = Handle { registry, wakers };

        Ok((driver, handle))
    }

    /// Poll events and dispatch.
    pub(crate) fn poll_events(&mut self) {
        self.poll
            .poll(&mut self.events, Some(Duration::from_micros(100)))
            .expect("failed to poll events");

        for event in self.events.iter() {
            if let Some(sender) = self.wakers.get(event.token().0) {
                let _ = sender.unbounded_send(event.clone());
            }
        }
    }
}

impl Handle {
    pub(crate) fn add_source<S: Source>(
        &self,
        source: &mut S,
        interests: Interest,
    ) -> io::Result<(Token, UnboundedReceiver<Event>)> {
        let (sender, receiver) = mpsc::unbounded();
        let token = Token(self.wakers.insert(sender).unwrap());

        self.registry.register(source, token, interests)?;

        Ok((token, receiver))
    }

    pub(crate) fn deregister_source<S: Source>(
        &self,
        source: &mut S,
        token: Token,
    ) -> io::Result<()> {
        self.registry.deregister(source)?;
        self.wakers.remove(token.0);

        Ok(())
    }
}
