/// Exposes some useful utilities around tracing.
/// Re-exports everything on tracing to avoid having to import tracing
/// everywhere along with this crate
pub mod custom_subscriber;
mod metrics;
pub use metrics::*;
pub use tracing::{self, *};
