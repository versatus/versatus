use primitives::{Address, SecretKey};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use secp256k1::{Message, Secp256k1};
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

pub fn generate_random_address() -> (SecretKey, Address) {
    let kp = Keypair::random();
    (kp.miner_kp.0, Address::new(kp.miner_kp.1))
}

pub fn generate_random_transaction(
    secret_key: primitives::SecretKey,
    from: Address,
    to: Address,
) -> Txn {
    type H = secp256k1::hashes::sha256::Hash;

    let secp = Secp256k1::new();
    let message = Message::from_hashed_data::<H>(b"vrrb");
    let signature = secp.sign_ecdsa(&message, &secret_key);

    Txn::new(NewTxnArgs {
        timestamp: 0,
        sender_address: from.to_string(),
        sender_public_key: from.public_key(),
        receiver_address: to.to_string(),
        token: None,
        amount: 100,
        signature,
        validators: None,
        nonce: 10,
    })
}

pub fn generate_random_valid_transaction() -> Txn {
    let (sender_secret_key, from) = generate_random_address();
    let (receiver_secret_key, to) = generate_random_address();

    type H = secp256k1::hashes::sha256::Hash;

    let secp = Secp256k1::new();
    let message = Message::from_hashed_data::<H>(b"vrrb");
    let signature = secp.sign_ecdsa(&message, &sender_secret_key);

    Txn::new(NewTxnArgs {
        timestamp: 0,
        sender_address: from.to_string(),
        sender_public_key: from.public_key(),
        receiver_address: to.to_string(),
        token: None,
        amount: 100,
        signature,
        validators: None,
        nonce: 10,
    })
}
