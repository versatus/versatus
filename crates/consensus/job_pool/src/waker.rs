use std::{
    task::Waker,
    thread::{self, Thread},
};

use once_cell::sync::Lazy;

pub fn empty_waker() -> Waker {
    static WAKER: Lazy<Waker> = Lazy::new(|| waker_fn::waker_fn(move || {}));

    WAKER.clone()
}

/// Creates a waker that unparks the current thread.
pub fn unpark_current_thread() -> Waker {
    thread_waker(thread::current())
}

/// Creates a waker that unparks a thread.
pub fn thread_waker(thread: Thread) -> Waker {
    waker_fn::waker_fn(move || thread.unpark())
}
