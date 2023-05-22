use std::net::SocketAddr;

use primitives::{Address, SecretKey};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use secp256k1::{Message, Secp256k1};
use vrrb_core::{
    claim::Claim,
    keypair::Keypair,
    txn::{NewTxnArgs, Txn},
};

// NOTE: this is used to generate random filenames so files created by tests
// don't get overwritten
pub fn _generate_random_string() -> String {
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

pub fn _generate_random_transaction(
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
        sender_address: from.clone(),
        sender_public_key: from.public_key(),
        receiver_address: to,
        token: None,
        amount: 100,
        signature,
        validators: None,
        nonce: 10,
    })
}

pub fn generate_random_valid_transaction() -> Txn {
    let (sender_secret_key, from) = generate_random_address();
    let (_, to) = generate_random_address();

    type H = secp256k1::hashes::sha256::Hash;

    let secp = Secp256k1::new();
    let message = Message::from_hashed_data::<H>(b"vrrb");
    let signature = secp.sign_ecdsa(&message, &sender_secret_key);

    Txn::new(NewTxnArgs {
        timestamp: 0,
        sender_address: from.clone(),
        sender_public_key: from.public_key(),
        receiver_address: to,
        token: None,
        amount: 100,
        signature,
        validators: None,
        nonce: 10,
    })
}

pub fn _generate_random_claim() -> Claim {
    let keypair = Keypair::random();
    let ip_address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    let public_key = keypair.get_miner_public_key().clone();
    let signature = Claim::signature_for_valid_claim(
        public_key,
        ip_address,
        keypair.get_miner_secret_key().secret_bytes().to_vec(),
    )
    .unwrap();
    Claim::new(
        public_key,
        Address::new(public_key),
        ip_address.clone(),
        signature,
    )
    .unwrap()
}
