use primitives::Address;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use vrrb_core::{
    keypair::Keypair,
    txn::{NewTxnArgs, Txn},
};

// NOTE: this is used to generate random filenames so files created by tests
// don't get overwritten
pub fn generate_random_string() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(30)
        .map(char::from)
        .collect()
}

pub fn generate_random_address() -> Address {
    let kp = Keypair::random();

    Address::new(kp.miner_kp.1)
}

pub fn generate_random_transaction(from: Address, to: Address) -> Txn {
    Txn::new(NewTxnArgs {
        sender_address: from.to_string(),
        sender_public_key: from.public_key_bytes(),
        receiver_address: to.to_string(),
        token: None,
        amount: 100,
        payload: None,
        signature: vec![],
        validators: None,
        nonce: 10,
    })
}

pub fn generate_random_valid_transaction() -> Txn {
    let from = generate_random_address();
    let to = generate_random_address();

    Txn::new(NewTxnArgs {
        sender_address: from.to_string(),
        sender_public_key: from.public_key_bytes(),
        receiver_address: to.to_string(),
        token: None,
        amount: 100,
        payload: None,
        signature: vec![],
        validators: None,
        nonce: 10,
    })
}
