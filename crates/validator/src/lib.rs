// pub mod mempool_processor;
pub mod result;
pub mod txn_validator;
pub mod validator_core;
pub mod validator_core_manager;

#[cfg(test)]
mod tests {

    use std::collections::HashMap;

    use primitives::AccountKeypair;
    use rand::{rngs::StdRng, Rng, SeedableRng};
    use vrrb_core::txn::*;

    use crate::{
        txn_validator::{StateSnapshot, TxnValidator},
        validator_core_manager::ValidatorCoreManager,
    };

    // TODO: Use proper txns when there will be proper txn validation
    // implemented
    fn random_string(rng: &mut StdRng) -> String {
        format!("{}", rng.gen::<u32>())
    }

    fn random_txn(rng: &mut StdRng) -> Txn {
        Txn::new(NewTxnArgs {
            sender_address: random_string(rng),
            sender_public_key: random_string(rng).as_bytes().to_vec(),
            receiver_address: random_string(rng),
            token: None,
            amount: 0,
            payload: Some(random_string(rng)),
            signature: random_string(rng).as_bytes().to_vec(),
            validators: Some(HashMap::<String, bool>::new()),
            nonce: 0,
        })
    }


    #[test]
    fn should_validate_a_list_of_invalid_transactions() {
        let mut valcore_manager = ValidatorCoreManager::new(TxnValidator::new(), 8).unwrap();
        let mut rng = rand::rngs::StdRng::from_seed([0; 32]);

        let mut batch = vec![];

        let mut rng = rand::rngs::StdRng::from_seed([0; 32]);
        for _ in 0..1000 {
            batch.push(random_txn(&mut rng));
        }

        let state_snapshot = StateSnapshot {
            accounts: HashMap::new(),
        };

        let target = batch
            .iter()
            .cloned()
            .map(|txn| {
                let account = txn.sender_address.clone();
                let err = Err(crate::txn_validator::TxnValidatorError::AccountNotFound(
                    account,
                ));

                (txn, err)
            })
            .collect();

        let validated = valcore_manager.validate(&state_snapshot, batch);
        assert_eq!(validated, target);
    }
}
