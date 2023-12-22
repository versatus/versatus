use bonsaidb::core::key::{KeyEncoding, KeyKind};
use secp256k1::{rand::rngs::OsRng, Secp256k1};
use sha2::Digest;
use sha3::Keccak256;
use std::str::FromStr;

use crate::{ByteVec, PublicKey, SecretKey};

pub type AddressBytes = [u8; 20];

/// Represents the lower 20 bytes
/// of a secp256k1 public key, hashed with sha256::digest
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Address(pub AddressBytes);

impl Address {
    pub fn new(public_key: PublicKey) -> Self {
        Self::from(public_key)
    }

    pub fn raw_address(&self) -> AddressBytes {
        self.0
    }

    #[deprecated]
    pub fn public_key_bytes(&self) -> ByteVec {
        // TODO: revisit later
        self.to_string().into_bytes()
    }
}

impl KeyEncoding for Address {
    type Error = std::convert::Infallible;
    const LENGTH: Option<usize> = None;
    fn as_ord_bytes(
        &self,
    ) -> std::result::Result<std::borrow::Cow<'_, [u8]>, std::convert::Infallible> {
        Ok(std::borrow::Cow::Borrowed(&self.0))
    }
    fn describe<Visitor>(visitor: &mut Visitor)
    where
        Visitor: bonsaidb::core::key::KeyVisitor,
    {
        visitor.visit_type(KeyKind::Bytes)
    }
}

impl serde::Serialize for Address {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.to_string().serialize(s)
    }
}

impl<'de> serde::Deserialize<'de> for Address {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        // NOTE: only deserialization from string is supported
        // TODO: figure out how to embed a custom error message here
        Address::from_str(&String::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}

impl Default for Address {
    fn default() -> Self {
        // NOTE: should never panic as it's a valid string
        // TODO: impl default null public keys to avoid this call to expect
        let pk = PublicKey::from_str("null-address").expect("can't create null address");
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
        // NOTE: does the same thing as before
        let encoded = hex::encode(self.0);
        write!(f, "0x{}", encoded)
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
