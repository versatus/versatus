//! A Job pool for running multiple tasks(Sync/Async) on a configurable group of
//! workers.

pub mod builder;
mod error;
mod poller;
pub mod pool;
mod task;
mod waker;
mod worker;
pub use crate::task::Task;
