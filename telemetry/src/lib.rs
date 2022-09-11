/// Exposes some useful utilities around tracing.
/// Re-exports everything on tracing to avoid having to import tracing
/// everywhere along with this crate
mod subscriber;
pub use subscriber::*;
pub use tracing::{self, *};
