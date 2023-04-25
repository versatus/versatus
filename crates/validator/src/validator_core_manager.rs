use std::collections::{HashMap, HashSet};

use primitives::Address;
use rayon::ThreadPoolBuilder;
use vrrb_core::{account::Account, claim::Claim, txn::Txn};

use crate::{
    claim_validator::ClaimValidator,
    result::{Result, ValidatorError},
    txn_validator::TxnValidator,
    validator_core::{Core, CoreId},
};

#[derive(Debug)]
pub struct ValidatorCoreManager {
    core_pool: rayon::ThreadPool,
}

impl ValidatorCoreManager {
    pub fn new(cores: usize) -> Result<Self> {
        let core_pool = ThreadPoolBuilder::new()
            .num_threads(cores)
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
                ClaimValidator,
            );
            valcore.process_transactions(account_state, batch)
        })
    }

    pub fn validate_claims(
        &mut self,
        claims: Vec<Claim>,
    ) -> HashSet<(Claim, crate::claim_validator::Result<()>)> {
        self.core_pool.install(|| {
            let valcore = Core::new(
                self.core_pool.current_thread_index().unwrap_or(0) as CoreId,
                TxnValidator::new(),
                ClaimValidator,
            );
            valcore.process_claims(claims)
        })
    }
}
