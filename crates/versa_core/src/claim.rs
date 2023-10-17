use std::net::SocketAddr;

use ethereum_types::U256;
use primitives::{Address, NodeId, PublicKey, SerializedSecretKey};
use serde::{Deserialize, Serialize};
/// a Module for creating, maintaining, and using a claim in the fair,
/// computationally inexpensive, collission proof, fully decentralized, fully
/// permissions Proof of Claim Miner Election algorithm
use serde_json;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    keypair::{KeyPairError, Keypair},
    ownable::Ownable,
    staking::{Stake, StakeError, StakeUpdate},
};

pub type Result<T> = std::result::Result<T, ClaimError>;

#[derive(Error, Debug)]
pub enum ClaimError {
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Invalid public key")]
    InvalidPublicKey,
    #[error("Details {0}")]
    Other(String),
}

impl From<KeyPairError> for ClaimError {
    fn from(error: KeyPairError) -> Self {
        match error {
            KeyPairError::InvalidSignature(_) => Self::InvalidSignature,
            KeyPairError::InvalidPublicKey => Self::InvalidPublicKey,
            KeyPairError::InvalidKey(_) => Self::Other(String::from("Invalid Secret Key")),
            _ => Self::Other(String::from("Failed to validate claim")),
        }
    }
}

/// The claim object that stores the key information used to mine blocks,
/// calculate whether or not you are an entitled miner, and to share with
/// network
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Claim {
    pub public_key: PublicKey,
    pub address: Address,
    pub hash: U256,
    pub eligibility: Eligibility,
    pub ip_address: SocketAddr,
    pub signature: String,
    pub node_id: NodeId,
    stake: u128,
    stake_txns: Vec<Stake>,
}

// TODO: Remove None variant and use Option<Eligibility>.
/// Node has privileges to be a Validator, Miner or None
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Eligibility {
    Validator,
    Miner,
    None,
}

impl std::fmt::Display for Eligibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Eligibility::Validator => write!(f, "Validator"),
            Eligibility::Miner => write!(f, "Miner"),
            Eligibility::None => write!(f, "None"),
        }
    }
}

impl Claim {
    /// Creates a new claim from a public key, address and nonce.
    pub fn new(
        public_key: PublicKey,
        address: Address,
        ip_address: SocketAddr,
        signature: String,
        node_id: NodeId,
    ) -> Result<Claim> {
        let mut hasher = Sha256::new();
        hasher.update(public_key.to_string());
        hasher.update(ip_address.to_string());
        let result = hasher.finalize();
        let hash = U256::from_big_endian(&result[..]);
        let mut msg_hash: Vec<u8> = Vec::new();
        hash.0.to_vec().iter().for_each(|x| {
            msg_hash.extend(x.to_le_bytes().iter());
        });
        return match Claim::is_valid_claim(
            msg_hash.as_slice(),
            signature.clone(),
            public_key.serialize().to_vec(),
        ) {
            Ok(_) => Ok(Claim {
                public_key,
                address,
                hash,
                eligibility: Eligibility::None,
                ip_address,
                signature,
                node_id,
                stake: 0,
                stake_txns: vec![],
            }),
            Err(e) => Err(e),
        };
    }

    /// The function generates a signature for creating valid claim using a
    /// public key, IP address, and secret key.
    ///
    /// Arguments:
    ///
    /// * `public_key`: The public key of the user making the claim.
    /// * `ip_address`: The `ip_address` parameter is of type `SocketAddr`,
    ///   which represents a socket
    /// address, including an IP address and a port number. It is used as part
    /// of the data that is hashed to create a signature for a valid claim.
    /// * `secret_key`: The secret key is a serialized version of the private
    ///   key used for ECDSA
    /// signing. It is needed to sign the hash of the public key and IP address
    /// to create a signature for a valid claim.
    ///
    /// Returns:
    ///
    /// a `Result<String>` which can either be an `Ok` variant containing the
    /// signature string for a valid claim or an `Err` variant containing an
    /// `InvalidClaimError` with details about the error encountered while
    /// creating the signature.
    pub fn signature_for_valid_claim(
        public_key: PublicKey,
        ip_address: SocketAddr,
        secret_key: SerializedSecretKey,
    ) -> Result<String> {
        let mut hasher = Sha256::new();
        hasher.update(public_key.to_string());
        hasher.update(ip_address.to_string());
        let result = hasher.finalize();
        let hash = U256::from_big_endian(&result[..]);
        let mut msg_hash: Vec<u8> = Vec::new();
        hash.0.to_vec().iter().for_each(|x| {
            msg_hash.extend(x.to_le_bytes().iter());
        });
        Keypair::ecdsa_sign(msg_hash.as_slice(), secret_key).map_err(ClaimError::from)
    }

    /// The function verifies the validity of a claim using ECDSA signature and
    /// public key.
    ///
    /// Arguments:
    ///
    /// * `msg_hash`: The `msg_hash` parameter is a slice of bytes representing
    ///   the hash of the message
    /// that was signed. This hash is typically generated using a cryptographic
    /// hash function such as SHA-256.
    /// * `signature`: The `signature` parameter is a string representing the
    ///   signature of a message. It
    /// is used in the `is_valid_claim` function to verify the authenticity of a
    /// claim.
    /// * `pub_key`: The `pub_key` parameter is a vector of bytes representing
    ///   the public key used for
    /// verifying the signature.
    ///
    /// Returns:
    ///
    /// a `Result` type, which can either be `Ok(())` if the signature is valid,
    /// or an `Err(ClaimError)` if the signature is invalid.
    pub fn is_valid_claim(msg_hash: &[u8], signature: String, pub_key: Vec<u8>) -> Result<()> {
        Keypair::verify_ecdsa_sign(signature, msg_hash, pub_key).map_err(ClaimError::from)
    }

    /// This function updates the IP address of a claim and verifies its
    /// validity using a signature and public key.
    ///
    /// Arguments:
    ///
    /// * `signature`: The signature is a cryptographic signature generated by
    ///   the claimant to prove
    /// their ownership of the public key and the IP address they are claiming.
    /// * `pub_key`: A vector of bytes representing the public key of the
    ///   claimant.
    /// * `ip_address`: The `ip_address` parameter is a `SocketAddr` type that
    ///   represents the IP address
    /// and port number of a network socket. It is used to update the IP address
    /// of a claim in the blockchain.
    ///
    /// Returns:
    ///
    /// a `Result<()>`, which means it either returns `Ok(())` if the function
    /// executes successfully or `ClaimError`.
    pub fn update_claim_socketaddr(
        &mut self,
        signature: String,
        public_key: PublicKey,
        ip_address: SocketAddr,
    ) -> Result<()> {
        let mut hasher = Sha256::new();
        hasher.update(public_key.to_string());
        hasher.update(ip_address.to_string());
        let result = hasher.finalize();
        let hash = U256::from_big_endian(&result[..]);
        let mut msg_hash: Vec<u8> = Vec::new();
        hash.0.to_vec().iter().for_each(|x| {
            msg_hash.extend(x.to_le_bytes().iter());
        });
        Claim::is_valid_claim(
            msg_hash.as_slice(),
            signature,
            public_key.serialize().to_vec(),
        )?;
        self.ip_address = ip_address;
        Ok(())
    }

    /// Uses XOR of the ClaimHash as a U256 against a block seed of u64
    /// U256 is represented as a [u64; 4] so we XOR each of the 4
    /// u64 values in the U256 against the block seed.
    pub fn get_election_result(&self, block_seed: u64) -> U256 {
        let mut xor_val = [0u64; 4];
        self.hash.0.iter().enumerate().for_each(|(idx, x)| {
            xor_val[idx] = x ^ block_seed;
        });

        U256(xor_val)
    }

    /// Takes a StakeUpdate enum and adds/withdrawals or slashes
    /// the given claim's stake. This method is used within the
    /// state module to update a claim that has a transaction
    /// pointing to it, and has been included in a certified
    /// convergence block.
    pub fn update_stake(&mut self, stake_txn: Stake) -> crate::staking::Result<()> {
        if !self.depositing_claim(&stake_txn) {
            return Err(StakeError::Other(
                "This claim is not the intended receiver of the stake transaction".to_string(),
            ));
        }

        if stake_txn.get_certificate().is_some() {
            let prev_stake = self.stake;
            self.stake_txns.push(stake_txn);
            self.stake = self.check_stake_utxo();

            if self.stake == prev_stake {
                self.stake_txns.pop();
            }

            return Ok(());
        }

        Err(StakeError::UncertifiedStake)
    }

    fn depositing_claim(&self, stake_txn: &Stake) -> bool {
        stake_txn.get_sender() == self.address
    }

    /// Checks the cumulative value of a nodes stake by calculating
    /// the UTXO of the stake transactions.
    fn check_stake_utxo(&self) -> u128 {
        self.stake_txns
            .iter()
            .fold(0u128, |mut acc, val| match val.get_amount() {
                StakeUpdate::Add(value) => {
                    if let Some(v) = acc.checked_add(value) {
                        acc = v;
                    }
                    acc
                },
                StakeUpdate::Withdrawal(value) => {
                    if let Some(v) = acc.checked_sub(value) {
                        acc = v;
                    }
                    acc
                },
                StakeUpdate::Slash(pct) => self.slash_calculator(pct, acc),
            })
    }

    /// Returns the slashed value of a nodes stake after a slashing
    /// event.
    fn slash_calculator(&self, pct: u8, value: u128) -> u128 {
        let slash = (value as f64) * (pct as f64 / 100f64);
        value - slash as u128
    }

    pub fn get_stake(&self) -> u128 {
        self.stake
    }

    pub fn get_stake_txns(&self) -> Vec<Stake> {
        self.stake_txns.clone()
    }

    #[deprecated(note = "Please use get_election_result")]
    pub fn get_pointer(&self, block_seed: u128) -> Option<u128> {
        let block_seed_hex = format!("{block_seed:x}");
        let block_seed_string_len = block_seed_hex.chars().count();
        let mut pointers = vec![];
        let mut hash_bytes = [0u8; 32];
        self.hash.to_big_endian(&mut hash_bytes);
        let hash_string = hex::encode(hash_bytes);

        block_seed_hex.chars().enumerate().for_each(|(idx, c)| {
            let res = hash_string.find(c);
            if let Some(n) = res {
                let n = n as u128;
                let n = n.checked_pow(idx as u32);
                if let Some(n) = n {
                    pointers.push(n);
                }
            }
        });

        if pointers.len() == block_seed_string_len {
            let pointer: u128 = pointers.iter().sum();
            Some(pointer)
        } else {
            None
        }
    }

    /// Converts a string representation of a claim to a `Claim` object
    pub fn from_string(claim_string: String) -> Claim {
        serde_json::from_str::<Claim>(&claim_string).unwrap()
    }

    /// Serializes a `Claim` into a Vector of bytes
    pub fn as_bytes(&self) -> Vec<u8> {
        serde_json::to_string(self).unwrap().as_bytes().to_vec()
    }

    /// Convert a byte representation of a claim to a `Claim` object
    pub fn from_bytes(data: &[u8]) -> Claim {
        serde_json::from_slice::<Claim>(data).unwrap()
    }

    /// get all the field names and stash them into a vector.
    pub fn get_field_names(&self) -> Vec<String> {
        vec![
            "pubkey".to_string(),
            "address".to_string(),
            "hash".to_string(),
            "eligible".to_string(),
        ]
    }

    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }

    pub fn address(&self) -> &Address {
        &self.address
    }

    pub fn eligibility(&self) -> &Eligibility {
        &self.eligibility
    }

    pub fn ip_address(&self) -> &SocketAddr {
        &self.ip_address
    }

    pub fn signature(&self) -> &String {
        &self.signature
    }

    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    pub fn public_key_owned(&self) -> PublicKey {
        self.public_key.clone()
    }

    pub fn address_owned(&self) -> Address {
        self.address.clone()
    }

    pub fn hash_owned(&self) -> U256 {
        self.hash
    }

    pub fn eligibility_owned(&self) -> Eligibility {
        self.eligibility.clone()
    }

    pub fn ip_address_owned(&self) -> SocketAddr {
        self.ip_address
    }

    pub fn signature_owned(&self) -> String {
        self.signature.clone()
    }

    pub fn node_id_owned(&self) -> NodeId {
        self.node_id.clone()
    }
}

/// Implements the Ownable trait on a claim
impl Ownable for Claim {
    type Pubkey = PublicKey;
    type SocketAddr = SocketAddr;

    fn get_public_key(&self) -> PublicKey {
        self.public_key
    }

    fn get_socket_addr(&self) -> Self::SocketAddr {
        self.ip_address
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keypair::KeyPair;

    #[test]
    fn should_create_new_claim() {
        let kp = KeyPair::random();
        let public_key = kp.miner_kp.1;
        let address = Address::new(public_key.clone());
        let ip_address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
        let mut hasher = Sha256::new();
        hasher.update(public_key.to_string().clone());
        hasher.update(ip_address.to_string().clone());
        let result = hasher.finalize();
        let hash = U256::from_big_endian(&result[..]);
        let signature = Claim::signature_for_valid_claim(
            public_key.clone(),
            ip_address,
            kp.get_miner_secret_key().secret_bytes().to_vec(),
        )
        .unwrap();
        let test_claim = Claim {
            public_key: public_key.clone(),
            address: address.clone(),
            hash,
            eligibility: Eligibility::None,
            ip_address: "127.0.0.1:8080".parse().unwrap(),
            signature: signature.clone(),
            node_id: NodeId::default(),
            stake: 0,
            stake_txns: vec![],
        };
        let claim = Claim::new(
            public_key,
            address,
            ip_address,
            signature,
            NodeId::default(),
        )
        .unwrap();
        assert_eq!(test_claim, claim);
    }

    #[test]
    fn update_ipaddress_in_claim() {
        let kp = KeyPair::random();
        let public_key = kp.miner_kp.1;
        let address = Address::new(public_key.clone());
        let ip_address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
        let mut hasher = Sha256::new();
        hasher.update(public_key.to_string().clone());
        hasher.update(ip_address.to_string().clone());
        let signature = Claim::signature_for_valid_claim(
            public_key.clone(),
            ip_address,
            kp.get_miner_secret_key().secret_bytes().to_vec(),
        )
        .unwrap();
        let mut claim = Claim::new(
            public_key.clone(),
            address,
            ip_address,
            signature,
            NodeId::default(),
        )
        .unwrap();

        let ip_address_new = "127.0.0.1:8081".parse::<SocketAddr>().unwrap();
        let mut hasher_new = Sha256::new();
        hasher_new.update(public_key.to_string().clone());
        hasher_new.update(ip_address_new.to_string().clone());
        let signature = Claim::signature_for_valid_claim(
            public_key.clone(),
            ip_address_new,
            kp.get_miner_secret_key().secret_bytes().to_vec(),
        )
        .unwrap();
        let status = claim.update_claim_socketaddr(signature, public_key.clone(), ip_address_new);
        assert!(status.is_ok());
        assert_eq!(claim.ip_address, ip_address_new);
    }

    #[test]
    fn stake_should_be_zero_by_default() {
        let kp = KeyPair::random();
        let public_key = kp.miner_kp.1;
        let address = Address::new(public_key.clone());
        let ip_address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
        let mut hasher = Sha256::new();
        hasher.update(public_key.to_string().clone());
        hasher.update(ip_address.to_string().clone());
        let signature = Claim::signature_for_valid_claim(
            public_key.clone(),
            ip_address,
            kp.get_miner_secret_key().secret_bytes().to_vec(),
        )
        .unwrap();
        let claim = Claim::new(
            public_key,
            address,
            ip_address,
            signature,
            NodeId::default(),
        )
        .unwrap();
        assert_eq!(0, claim.get_stake());
    }

    #[test]
    fn stake_txns_should_be_empty_by_default() {
        let kp = KeyPair::random();
        let public_key = kp.miner_kp.1;
        let address = Address::new(public_key.clone());
        let ip_address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
        let mut hasher = Sha256::new();
        hasher.update(public_key.to_string().clone());
        hasher.update(ip_address.to_string().clone());
        let signature = Claim::signature_for_valid_claim(
            public_key.clone(),
            ip_address,
            kp.get_miner_secret_key().secret_bytes().to_vec(),
        )
        .unwrap();
        let claim = Claim::new(
            public_key,
            address,
            ip_address,
            signature,
            NodeId::default(),
        )
        .unwrap();
        assert_eq!(0, claim.get_stake_txns().len());
    }

    #[test]
    fn should_return_election_result() {
        let kp = KeyPair::random();
        let public_key = kp.miner_kp.1;
        let address = Address::new(public_key.clone());
        let ip_address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
        let mut hasher = Sha256::new();
        hasher.update(public_key.to_string().clone());
        hasher.update(ip_address.to_string().clone());
        let result = hasher.finalize();
        let hash = U256::from_big_endian(&result[..]);
        let signature = Claim::signature_for_valid_claim(
            public_key.clone(),
            ip_address,
            kp.get_miner_secret_key().secret_bytes().to_vec(),
        )
        .unwrap();

        let mut xor_val = [0u64; 4];
        let seed: u64 = u64::default();
        hash.clone().0.iter().enumerate().for_each(|(idx, x)| {
            xor_val[idx] = x ^ seed;
        });

        let test_election_result = U256(xor_val);

        let claim = Claim::new(
            public_key,
            address,
            ip_address,
            signature,
            NodeId::default(),
        )
        .unwrap();

        let election_result = claim.get_election_result(seed);

        assert_eq!(test_election_result, election_result)
    }

    #[test]
    fn should_reject_uncertified_stake_from_claim() {
        let kp = KeyPair::random();
        let public_key = kp.miner_kp.1;
        let address = Address::new(public_key.clone());
        let ip_address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
        let mut hasher = Sha256::new();
        hasher.update(public_key.to_string().clone());
        hasher.update(ip_address.to_string().clone());
        let signature = Claim::signature_for_valid_claim(
            public_key.clone(),
            ip_address,
            kp.get_miner_secret_key().secret_bytes().to_vec(),
        )
        .unwrap();
        let mut claim = Claim::new(
            public_key,
            address.clone(),
            ip_address,
            signature,
            NodeId::default(),
        )
        .unwrap();

        let amount = StakeUpdate::Add(10_000u128);

        let stake = Stake::new(
            amount,
            kp.miner_kp.0.clone(),
            kp.miner_kp.1.clone(),
            address,
            None,
        )
        .unwrap();

        assert!(claim.update_stake(stake).is_err());
        assert_eq!(claim.get_stake(), 0u128);
    }

    #[test]
    fn should_add_stake_to_claim() {
        let kp = KeyPair::random();
        let public_key = kp.miner_kp.1;
        let address = Address::new(public_key.clone());
        let ip_address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
        let mut hasher = Sha256::new();
        hasher.update(public_key.to_string().clone());
        hasher.update(ip_address.to_string().clone());
        let signature = Claim::signature_for_valid_claim(
            public_key.clone(),
            ip_address,
            kp.get_miner_secret_key().secret_bytes().to_vec(),
        )
        .unwrap();
        let mut claim = Claim::new(
            public_key,
            address.clone(),
            ip_address,
            signature,
            NodeId::default(),
        )
        .unwrap();

        let amount = StakeUpdate::Add(10_000u128);

        let mut stake = Stake::new(
            amount,
            kp.miner_kp.0.clone(),
            kp.miner_kp.1.clone(),
            address,
            None,
        )
        .unwrap();

        stake.certify((vec![0; 96], vec![0; 96])).unwrap();

        assert!(claim.update_stake(stake).is_ok());
        assert_eq!(claim.get_stake(), 10_000u128);
    }

    #[test]
    fn should_add_stake_txn_to_claim() {
        let kp = KeyPair::random();
        let public_key = kp.miner_kp.1;
        let address = Address::new(public_key.clone());
        let ip_address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
        let mut hasher = Sha256::new();
        hasher.update(public_key.to_string().clone());
        hasher.update(ip_address.to_string().clone());
        let signature = Claim::signature_for_valid_claim(
            public_key.clone(),
            ip_address,
            kp.get_miner_secret_key().secret_bytes().to_vec(),
        )
        .unwrap();
        let mut claim = Claim::new(
            public_key,
            address.clone(),
            ip_address.clone(),
            signature,
            NodeId::default(),
        )
        .unwrap();

        let amount = StakeUpdate::Add(10_000u128);

        let mut stake = Stake::new(
            amount,
            kp.miner_kp.0.clone(),
            kp.miner_kp.1.clone(),
            address,
            None,
        )
        .unwrap();

        stake.certify((vec![0; 96], vec![0; 96])).unwrap();

        assert!(claim.update_stake(stake).is_ok());
        assert_eq!(claim.get_stake_txns().len(), 1);
    }

    #[test]
    fn should_withdrawal_stake_from_claim() {
        let kp = KeyPair::random();
        let public_key = kp.miner_kp.1;
        let address = Address::new(public_key.clone());
        let ip_address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
        let mut hasher = Sha256::new();
        hasher.update(public_key.to_string().clone());
        hasher.update(ip_address.to_string().clone());
        let signature = Claim::signature_for_valid_claim(
            public_key.clone(),
            ip_address,
            kp.get_miner_secret_key().secret_bytes().to_vec(),
        )
        .unwrap();
        let mut claim = Claim::new(
            public_key,
            address.clone(),
            ip_address,
            signature,
            NodeId::default(),
        )
        .unwrap();
        let amount = StakeUpdate::Add(10_000u128);
        let mut stake = Stake::new(
            amount,
            kp.miner_kp.0.clone(),
            kp.miner_kp.1.clone(),
            address.clone(),
            None,
        )
        .unwrap();

        stake.certify((vec![0; 96], vec![0; 96])).unwrap();

        assert!(claim.update_stake(stake).is_ok());
        assert_eq!(claim.get_stake(), 10_000u128);

        let amount = StakeUpdate::Withdrawal(5_000u128);

        let mut stake = Stake::new(
            amount,
            kp.miner_kp.0.clone(),
            kp.miner_kp.1.clone(),
            address.clone(),
            None,
        )
        .unwrap();

        stake.certify((vec![0; 96], vec![0; 96])).unwrap();

        assert!(claim.update_stake(stake).is_ok());
        assert_eq!(claim.get_stake(), 5_000u128);
        assert_eq!(claim.get_stake_txns().len(), 2);
    }

    #[test]
    fn should_do_nothing_withdrawal_stake_from_claim_with_no_stake() {
        let kp = KeyPair::random();
        let public_key = kp.miner_kp.1;
        let address = Address::new(public_key.clone());
        let ip_address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
        let mut hasher = Sha256::new();
        hasher.update(public_key.to_string().clone());
        hasher.update(ip_address.to_string().clone());
        let signature = Claim::signature_for_valid_claim(
            public_key.clone(),
            ip_address,
            kp.get_miner_secret_key().secret_bytes().to_vec(),
        )
        .unwrap();
        let mut claim = Claim::new(
            public_key,
            address.clone(),
            ip_address,
            signature,
            NodeId::default(),
        )
        .unwrap();

        let amount = StakeUpdate::Withdrawal(5_000u128);
        let mut stake = Stake::new(
            amount,
            kp.miner_kp.0.clone(),
            kp.miner_kp.1.clone(),
            address.clone(),
            None,
        )
        .unwrap();

        stake.certify((vec![0; 96], vec![0; 96])).unwrap();

        assert!(claim.update_stake(stake).is_ok());
        assert_eq!(claim.get_stake(), 0u128);
        assert_eq!(claim.get_stake_txns().len(), 0);
    }

    #[test]
    fn should_slash_stake_from_claim() {
        let kp = KeyPair::random();
        let public_key = kp.miner_kp.1;
        let address = Address::new(public_key.clone());
        let ip_address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
        let mut hasher = Sha256::new();
        hasher.update(public_key.to_string().clone());
        hasher.update(ip_address.to_string().clone());
        let signature = Claim::signature_for_valid_claim(
            public_key.clone(),
            ip_address,
            kp.get_miner_secret_key().secret_bytes().to_vec(),
        )
        .unwrap();
        let mut claim = Claim::new(
            public_key,
            address.clone(),
            ip_address,
            signature,
            NodeId::default(),
        )
        .unwrap();

        let amount = StakeUpdate::Add(10_000u128);

        let mut stake = Stake::new(
            amount,
            kp.miner_kp.0.clone(),
            kp.miner_kp.1.clone(),
            address.clone(),
            None,
        )
        .unwrap();

        stake.certify((vec![0; 96], vec![0; 96])).unwrap();

        assert!(claim.update_stake(stake).is_ok());
        assert_eq!(claim.get_stake(), 10_000u128);

        let amount = StakeUpdate::Slash(25u8);

        let mut stake = Stake::new(
            amount,
            kp.miner_kp.0.clone(),
            kp.miner_kp.1.clone(),
            address.clone(),
            None,
        )
        .unwrap();

        stake.certify((vec![0; 96], vec![0; 96])).unwrap();

        assert!(claim.update_stake(stake).is_ok());
        assert_eq!(claim.get_stake(), 7_500u128);
        assert_eq!(claim.get_stake_txns().len(), 2);
    }

    #[test]
    fn should_do_nothing_slash_stake_from_claim_with_no_stake() {
        let kp = KeyPair::random();
        let public_key = kp.miner_kp.1;
        let address = Address::new(public_key.clone());
        let ip_address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
        let mut hasher = Sha256::new();
        hasher.update(public_key.to_string().clone());
        hasher.update(ip_address.to_string().clone());
        let signature = Claim::signature_for_valid_claim(
            public_key.clone(),
            ip_address,
            kp.get_miner_secret_key().secret_bytes().to_vec(),
        )
        .unwrap();
        let mut claim = Claim::new(
            public_key,
            address.clone(),
            ip_address,
            signature,
            NodeId::default(),
        )
        .unwrap();

        let amount = StakeUpdate::Slash(25u8);
        let mut stake = Stake::new(
            amount,
            kp.miner_kp.0.clone(),
            kp.miner_kp.1.clone(),
            address.clone(),
            None,
        )
        .unwrap();

        stake.certify((vec![0; 96], vec![0; 96])).unwrap();

        assert!(claim.update_stake(stake).is_ok());
        assert_eq!(claim.get_stake(), 0u128);
        assert_eq!(claim.get_stake_txns().len(), 0);
    }

    #[test]
    fn should_calculate_utxo_of_claim_stake() {
        let kp = KeyPair::random();
        let public_key = kp.miner_kp.1;
        let address = Address::new(public_key.clone());
        let ip_address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
        let mut hasher = Sha256::new();
        hasher.update(public_key.to_string().clone());
        hasher.update(ip_address.to_string().clone());
        let signature = Claim::signature_for_valid_claim(
            public_key.clone(),
            ip_address,
            kp.get_miner_secret_key().secret_bytes().to_vec(),
        )
        .unwrap();
        let mut claim = Claim::new(
            public_key,
            address.clone(),
            ip_address,
            signature,
            NodeId::default(),
        )
        .unwrap();

        let amount = StakeUpdate::Add(10_000u128);

        let mut stake = Stake::new(
            amount,
            kp.miner_kp.0.clone(),
            kp.miner_kp.1.clone(),
            address.clone(),
            None,
        )
        .unwrap();

        stake.certify((vec![0; 96], vec![0; 96])).unwrap();

        assert!(claim.update_stake(stake).is_ok());
        assert_eq!(claim.get_stake(), 10_000u128);

        let amount = StakeUpdate::Add(80_000u128);

        let mut stake = Stake::new(
            amount,
            kp.miner_kp.0.clone(),
            kp.miner_kp.1.clone(),
            address.clone(),
            None,
        )
        .unwrap();

        stake.certify((vec![0; 96], vec![0; 96])).unwrap();

        assert!(claim.update_stake(stake).is_ok());
        assert_eq!(claim.get_stake(), 90_000u128);
        assert_eq!(claim.get_stake_txns().len(), 2);
    }
}
