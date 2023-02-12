use std::collections::HashSet;

use rayon::ThreadPoolBuilder;
use vrrb_core::txn::Txn;

use crate::{
    result::{Result, ValidatorError},
    txn_validator::{StateSnapshot, TxnValidator},
    validator_core::{Core, CoreError},
};

#[derive(Debug)]
pub struct ValidatorCoreManager {
    // core_pool: Vec<Core>,
    core_pool: rayon::ThreadPool,
}

impl ValidatorCoreManager {
    pub fn new(validator: TxnValidator, cores: usize) -> Result<Self> {
        let core_pool = ThreadPoolBuilder::new()
            .num_threads(8)
            .build()
            .map_err(|err| {
                ValidatorError::Other(format!("Failed to create validator core pool: {}", err))
            })?;

        Ok(Self { core_pool })
    }

    pub fn validate(
        &mut self,
        state_snapshot: &StateSnapshot,
        batch: Vec<Txn>,
    ) -> HashSet<(Txn, crate::txn_validator::Result<()>)> {
        // ) -> HashSet<(Txn, bool)> {
        self.core_pool.install(|| {
            let valcore = Core::new(1, TxnValidator::new());

            valcore.process_transactions(state_snapshot, batch)
        })
    }
}
