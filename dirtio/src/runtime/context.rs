use crate::runtime::scheduler::handle::Handle;

use std::cell::RefCell;

thread_local! {
    /// Handle to the runtime of current thread.
    static CONTEXT: RefCell<Option<Handle>> = RefCell::new(None);
}

pub(crate) fn current() -> Handle {
    CONTEXT.with(|r| {
        r.borrow()
            .as_ref()
            .expect("called outside of context")
            .clone()
    })
}

pub(crate) fn set_current(handle: Handle) {
    CONTEXT.with(|r| r.borrow_mut().replace(handle));
}
