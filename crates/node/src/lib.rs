pub mod result;

pub mod node;
mod runtime_component;
mod runtime_module;

pub(crate) mod network;

pub(crate) mod node_health_report;
pub(crate) mod runtime;

pub mod test_utils;

pub use result::*;
pub use runtime::*;
pub use runtime_component::*;
pub use runtime_module::*;

pub use crate::node::*;

/// The maximum size in kilobytes of transactions that can be in the mempool at
/// any given time.
pub(crate) const MEMPOOL_THRESHOLD_SIZE: usize = 2048;

/// Represents the number of packets that can be lost and still be able to
/// reconstruct the message.
pub(crate) const DEFAULT_ERASURE_COUNT: u32 = 100;
