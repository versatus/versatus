use std::collections::{HashMap, HashSet};

use primitives::Address;
use rayon::ThreadPoolBuilder;
use vrrb_core::{account::Account, txn::Txn};

use crate::{
    result::{Result, ValidatorError},
    txn_validator::TxnValidator,
    validator_core::{Core, CoreError, CoreId},
};


#[derive(Debug)]
pub struct ValidatorCoreManager {
    // core_pool: Vec<Core>,
    core_pool: rayon::ThreadPool,
}

impl ValidatorCoreManager {
    pub fn new(validator: TxnValidator, cores: usize) -> Result<Self> {
        let core_pool = ThreadPoolBuilder::new()
            .num_threads(2)
            .build()
            .map_err(|err| {
                ValidatorError::Other(format!("Failed to create validator core pool: {}", err))
            })?;

        Ok(Self { core_pool })
    }

    pub fn validate(
        &mut self,
        account_state: &HashMap<Address, Account>,
        batch: Vec<Txn>,
    ) -> HashSet<(Txn, crate::txn_validator::Result<()>)> {
        // ) -> HashSet<(Txn, bool)> {
        self.core_pool.install(|| {
            let valcore = Core::new(
                self.core_pool.current_thread_index().unwrap_or(0) as CoreId,
                TxnValidator::new(),
            );

            valcore.process_transactions(account_state, batch)
        })
    }
}
