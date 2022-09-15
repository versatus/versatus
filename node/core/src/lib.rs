pub mod handler;
/// Deprecated: use node::core instead
pub mod node;

pub mod core {
    // rename node.rs to core.rs once other refactoring efforts are complete
    pub use super::node::*;
}
