use wasmer::wasmparser::Operator;
use wasmer_middlewares::Metering;

/// A convenience wrapper for creating a new `wasmer_middlewares::Metering`.
pub struct MeteringConfig<F: Fn(&Operator) -> u64 + Send + Sync> {
    /// Initial limit of points.
    initial_limit: u64,
    /// Function that maps each operator to a cost in "points".
    cost_function: F,
}
impl<F> MeteringConfig<F>
where
    F: Fn(&Operator) -> u64 + Send + Sync,
{
    pub fn new(initial_limit: u64, cost_function: F) -> Self {
        Self {
            initial_limit,
            cost_function,
        }
    }
    pub(crate) fn into_metering(self) -> Metering<F> {
        Metering::new(self.initial_limit, self.cost_function)
    }
}
