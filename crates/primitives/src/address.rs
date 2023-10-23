use secp256k1::{rand::rngs::OsRng, Secp256k1};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use sha3::Keccak256;
use std::str::FromStr;

use crate::{ByteVec, PublicKey, SecretKey};

/// Represents the lower 20 bytes
/// of a secp256k1 public key,
/// hashed with sha256::digest
//pub struct Address(PublicKey);
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct Address(pub [u8; 20]);

impl Address {
    pub fn new(public_key: PublicKey) -> Self {
        Self::from(public_key)
    }

    pub fn raw_address(&self) -> [u8; 20] {
        self.0
    }

    #[deprecated]
    pub fn public_key_bytes(&self) -> ByteVec {
        // TODO: revisit later
        self.to_string().into_bytes()
    }
}

impl Default for Address {
    fn default() -> Self {
        // NOTE: should never panic as it's a valid string
        // TODO: impl default null public keys to avoid this call to expect
        let pk = PublicKey::from_str("null-address").expect("cant create null address");
        Self::from(pk)
    }
}

impl From<String> for Address {
    fn from(s: String) -> Self {
        Self::from_str(&s).unwrap_or_default()
    }
}

impl From<PublicKey> for Address {
    fn from(item: PublicKey) -> Self {
        let mut hasher = Keccak256::new();
        let pk_bytes = item.serialize_uncompressed();
        let apk_bytes = &pk_bytes[1..];
        hasher.update(apk_bytes);
        let hash = hasher.finalize();
        let address_hash_slice = hash[(hash.len() - 20)..].to_vec();
        let mut address_bytes = [0u8; 20];
        address_bytes.copy_from_slice(&address_hash_slice);
        Address(address_bytes)
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hex_address = format!(
            "0x{}",
            self.0
                .iter()
                .map(|b| { format!("{:02x}", b) })
                .collect::<String>()
        );

        f.write_str(&hex_address)
    }
}

impl FromStr for Address {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 42 || !s.starts_with("0x") {
            return Err(hex::FromHexError::OddLength);
        }

        let address_bytes = hex::decode(&s[2..])?;

        if address_bytes.len() != 20 {
            return Err(hex::FromHexError::OddLength);
        }

        let mut address = [0u8; 20];
        address.copy_from_slice(&address_bytes);

        Ok(Address(address))
    }
}

pub type AccountKeypair = (secp256k1::SecretKey, secp256k1::PublicKey);

pub fn generate_account_keypair() -> AccountKeypair {
    let secp = Secp256k1::new();
    secp.generate_keypair(&mut OsRng)
}

pub fn generate_mock_account_keypair() -> AccountKeypair {
    type H = secp256k1::hashes::sha256::Hash;

    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_hashed_data::<H>(b"vrrb");
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    (secret_key, public_key)
}
