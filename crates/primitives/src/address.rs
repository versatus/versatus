use secp256k1::{rand::rngs::OsRng, Secp256k1};

use crate::PublicKey;

/// Represents a secp256k1 public key, hashed with sha256::digest
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Address(PublicKey);

impl Address {
    pub fn new(public_key: PublicKey) -> Self {
        Self(public_key)
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.to_string())
    }
}

pub type AccountKeypair = (secp256k1::SecretKey, secp256k1::PublicKey);

pub fn generate_account_keypair() -> AccountKeypair {
    let secp = Secp256k1::new();
    secp.generate_keypair(&mut OsRng)
}
