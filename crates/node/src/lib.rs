extern crate core;

mod node;
mod node_type;
pub mod result;
mod runtime;
mod runtime_module;
pub mod services;
pub mod test_utils;

pub use node_type::*;
pub use result::*;
pub use runtime::*;
pub use runtime_module::*;
pub use services::*;
use vrrb_core::event_router::DirectedEvent;

pub use crate::node::*;

pub(crate) type EventBroadcastSender = tokio::sync::mpsc::UnboundedSender<DirectedEvent>;

/// The maximum size in kilobytes of transactions that can be in the mempool at
/// any given time.
// pub(crate) const MEMPOOL_THRESHOLD_SIZE: usize = 2048;
// pub(crate) const MEMPOOL_THRESHOLD_SIZE: usize = 512;
pub(crate) const MEMPOOL_THRESHOLD_SIZE: usize = 28;
