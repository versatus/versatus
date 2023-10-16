// pub mod mempool_processor;
pub mod claim_validator;
pub mod result;
pub mod txn_validator;
pub mod validator_core;
pub mod validator_core_manager;

#[cfg(test)]
mod tests {

    use std::collections::HashMap;

    use primitives::{Address, Signature};
    use rand::{rngs::StdRng, Rng};
    use secp256k1::ecdsa;
    use vrrb_core::transactions::{NewTransferArgs, TransactionKind, Transfer};
    use vrrb_core::{account::Account, keypair::KeyPair};

    use crate::validator_core_manager::ValidatorCoreManager;

    // TODO: Use proper txns when there will be proper txn validation
    // implemented
    fn _random_string(rng: &mut StdRng) -> String {
        format!("{}", rng.gen::<u32>())
    }

    fn mock_txn_signature() -> Signature {
        ecdsa::Signature::from_compact(&[
            0xdc, 0x4d, 0xc2, 0x64, 0xa9, 0xfe, 0xf1, 0x7a, 0x3f, 0x25, 0x34, 0x49, 0xcf, 0x8c,
            0x39, 0x7a, 0xb6, 0xf1, 0x6f, 0xb3, 0xd6, 0x3d, 0x86, 0x94, 0x0b, 0x55, 0x86, 0x82,
            0x3d, 0xfd, 0x02, 0xae, 0x3b, 0x46, 0x1b, 0xb4, 0x33, 0x6b, 0x5e, 0xcb, 0xae, 0xfd,
            0x66, 0x27, 0xaa, 0x92, 0x2e, 0xfc, 0x04, 0x8f, 0xec, 0x0c, 0x88, 0x1c, 0x10, 0xc4,
            0xc9, 0x42, 0x8f, 0xca, 0x69, 0xc1, 0x32, 0xa2,
        ])
        .unwrap()
    }

    fn random_txn() -> TransactionKind {
        let sender_kp = KeyPair::random();
        let recv_kp = KeyPair::random();

        let sender_address = Address::new(sender_kp.get_miner_public_key().clone());
        let recv_address = Address::new(recv_kp.get_miner_public_key().clone());

        TransactionKind::Transfer(Transfer::new(NewTransferArgs {
            timestamp: 0,
            sender_address: sender_address.clone(),
            sender_public_key: sender_kp.get_miner_public_key().clone(),
            receiver_address: recv_address.clone(),
            token: None,
            amount: 0,
            signature: mock_txn_signature(),
            validators: Some(HashMap::<String, bool>::new()),
            nonce: 0,
        }))
    }

    #[test]
    #[ignore = "https://github.com/versatus/versatus/issues/472"]
    fn should_validate_a_list_of_invalid_transactions() {
        // let mut valcore_manager = ValidatorCoreManager::new(8).unwrap();

        // let mut batch = vec![];

        // for _ in 0..1000 {
        //     batch.push(random_txn());
        // }

        // let account_state: HashMap<Address, Account> = HashMap::new();

        // let target = batch
        //     .iter()
        //     .cloned()
        //     .map(|txn| {
        //         let err = Err(crate::txn_validator::TxnValidatorError::SenderAddressIncorrect);
        //         (txn, err)
        //     })
        //     .collect();

        // let validated = valcore_manager.validate(&account_state, batch);
        // assert_eq!(validated, target);
    }
}
