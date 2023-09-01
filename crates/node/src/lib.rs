pub mod result;

pub mod node;
mod runtime_component;
mod runtime_module;

pub(crate) mod api;
pub(crate) mod consensus;
pub(crate) mod data_store;
pub(crate) mod indexer_module;
pub(crate) mod mining_module;
pub(crate) mod network;
pub(crate) mod runtime;
pub(crate) mod state_manager;
pub(crate) mod state_reader;
pub(crate) mod ui;

pub mod test_utils;

pub use result::*;
pub use runtime::*;
pub use runtime_component::*;
pub use runtime_module::*;

pub use crate::node::*;

/// Represents the number of packets that can be lost and still be able to
/// reconstruct the message.
pub(crate) const DEFAULT_ERASURE_COUNT: u32 = 100;
