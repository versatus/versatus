use wasmer::wasmparser::Operator;
use wasmer_middlewares::Metering;

// This function will be called for each `Operator` encountered during
// the Wasm module execution. It should return the cost of the operator
// that it received as it first argument.
pub fn cost_function(_operator: &Operator) -> u64 {
    // Cost fn from wasmer examples:
    //
    // match operator {
    //     Operator::LocalGet { .. } | Operator::I32Const { .. } => 1,
    //     Operator::I32Add { .. } => 2,
    //     _ => 0,
    // }

    0 /* for now we just return 1 regardless of the instruction */
}

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
