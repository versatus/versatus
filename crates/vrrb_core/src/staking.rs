use primitives::{Address, PayloadHash, QuorumPublicKey, Signature};
use secp256k1::Message;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use utils::hash_data;

use crate::keypair::{MinerPk, MinerSk};

/// Represents a byte array that can be converted into a
/// ThresholdSignature
pub type Certificate = (Vec<u8>, PayloadHash);
pub const MIN_STAKE_FARMER: u128 = 10_000;
pub const MIN_STAKE_VALIDATOR: u128 = 50_000;

pub type Result<T> = std::result::Result<T, StakeError>;

#[derive(Debug, Error, PartialEq, Clone, Serialize, Deserialize, Eq)]
pub enum StakeError {
    #[error("StakeError: The payload was not able to be converted into a valid message")]
    InvalidPayload,
    #[error("StakeError: The signature was invalid for the given message and public key")]
    InvalidSignature,
    #[error("StakError: The stake transaction has not been certified")]
    UncertifiedStake,
    #[error("StakError: The certificate is invalid")]
    InvalidCertificate,
    #[error("StakeError: {0}")]
    Other(String),
}

/// Provides an enum with the 3 different types of StakeUpdates that
/// are possible, and a inner value which is the amount (for Add and
/// Withdrawal variants) and the percent to slash (for Slash) variant.
///
/// ```
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
/// pub enum StakeUpdate {
///     Add(u128),
///     Withdrawal(u128),
///     Slash(u8),
/// }
/// ```
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StakeUpdate {
    Add(u128),
    Withdrawal(u128),
    Slash(u8),
}

/// A struct thatt defines a stake, includes the public key (which
/// can be converted into an address) an amount, which is an instance
/// of the `StakeUpdate` enum, a timestamp to sequence it in the
/// `StakeTxns` field of the claim, and a signature to verify it indeed
/// came from the publickey in question.
///
/// Also includes an optional address, if `None` is provided then
/// it is assumed the stake is directed to the claim address associated
/// with the pubkey field in this struct. If `Some` is provided then it
/// is assumed that the stake is being delegated to another node.
///
/// ```
/// use primitives::{Address, Signature};
/// use serde::{Deserialize, Serialize};
/// use vrrb_core::{
///     keypair::{MinerPk, MinerSk},
///     staking::StakeUpdate,
/// };
///
/// #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
/// pub struct Stake {
///     pubkey: MinerPk,
///     from: Address,
///     to: Option<Address>,
///     amount: StakeUpdate,
///     timestamp: i64,
///     signature: Signature,
/// };
/// ```
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Stake {
    pubkey: MinerPk,
    from: Address,
    to: Option<Address>,
    amount: StakeUpdate,
    timestamp: i64,
    signature: Signature,
    validator_quorum_key: QuorumPublicKey,
    certificate: Option<Certificate>,
}

impl Stake {
    pub const MAX: u128 = 100_000;
    pub const MIN: u128 = 10_000;

    /// Creates a new Stake which is a type of verifiable transaction
    /// that is to be sent to the validator network for verification
    /// and ceritification, as well as inclusion in a block.
    ///
    /// ```
    /// use primitives::Address;
    /// use vrrb_core::{
    ///     keypair::{KeyPair, MinerSk},
    ///     staking::{Stake, StakeUpdate},
    /// };
    ///
    /// let keypair = KeyPair::random();
    /// let sk = keypair.miner_kp.0.clone();
    /// let pk = keypair.miner_kp.1.clone();
    /// let amount = StakeUpdate::Add(10_000u128);
    /// let from = Address::new(pk.clone());
    ///
    /// let stake = Stake::new(amount, sk, pk, from, None);
    ///
    /// println!("{:?}", stake);
    /// ```
    pub fn new(
        amount: StakeUpdate,
        sk: MinerSk,
        pk: MinerPk,
        from: Address,
        to: Option<Address>,
    ) -> Option<Self> {
        let timestamp = chrono::Utc::now().timestamp();
        let payload = hash_data!(pk, from, to, amount, timestamp);
        if let Ok(message) = Message::from_slice(&payload) {
            let signature = sk.sign_ecdsa(message);

            return Some(Stake {
                pubkey: pk,
                from,
                to,
                amount,
                timestamp,
                signature,
                validator_quorum_key: vec![],
                certificate: None,
            });
        }

        None
    }

    /// This function returns the validator quorum public key which was used to
    /// certify the stake .
    ///
    /// Returns:
    ///
    /// The `get_quorum_key` function is returning a clone of the
    /// `validator_quorum_key` field of the current object, which is of type
    /// `QuorumPublicKey`.
    pub fn get_quorum_key(&self) -> QuorumPublicKey {
        self.validator_quorum_key.clone()
    }

    /// returns the Stake public key which is used to verify
    /// the signature of the Stake transaction
    pub fn get_pubkey(&self) -> MinerPk {
        self.pubkey
    }

    /// Returns the address from which the stake is to be
    /// posted or withdrawn to.
    pub fn get_sender(&self) -> Address {
        self.from.clone()
    }

    /// Returns the receiving Claim address. This is either
    /// an address stake is being delegated to, or is None
    /// in which case the receiver is the same as the Sender
    /// i.e. a node is putting a stake in its *own* claim.
    pub fn get_receiver(&self) -> Address {
        if let Some(address) = &self.to {
            return address.clone();
        }

        self.from.clone()
    }

    /// Returns the StakeUpdate enum variant for this particular
    /// instance.
    pub fn get_amount(&self) -> StakeUpdate {
        self.amount
    }

    /// Returns the timestamp of this particular instance
    pub fn get_timestamp(&self) -> i64 {
        self.timestamp
    }

    /// Returns the ecdsa signature of the particular instance
    pub fn get_signature(&self) -> Signature {
        self.signature
    }

    /// Returns t he payload which is the hashed data
    /// that is signed using the initiators secret key.
    /// This is used to reconstruct the message and verify the
    /// signature using the provided public key.
    pub fn get_payload(&self) -> Vec<u8> {
        hash_data!(self.pubkey, self.from, self.to, self.amount, self.timestamp).to_vec()
    }

    /// Returns the instances certificate, if there is one.
    /// The certificate is a Threshold Signature that Farmer
    /// nodes use to ensure a given threshold of validators have
    /// agreed upon the validity of a given transaction, in this
    /// case, the Stake transaction instance
    pub fn get_certificate(&self) -> Option<Certificate> {
        self.certificate.clone()
    }

    /// Adds a certificate to the instance.
    pub fn certify(&mut self, certificate: Certificate) -> Result<()> {
        const VALID_CERTIFICATE_LENGTH: usize = 96;
        if certificate.0.len() != VALID_CERTIFICATE_LENGTH {
            return Err(StakeError::InvalidCertificate);
        }

        self.certificate = Some(certificate);

        Ok(())
    }

    /// Verifies the signature of the StakeTransaction
    pub fn verify(&self) -> Result<()> {
        let payload = self.get_payload();
        if let Ok(message) = Message::from_slice(&payload) {
            self.signature
                .verify(&message, &self.get_pubkey())
                .map_err(|_| StakeError::InvalidSignature)?;
            return Ok(());
        }

        Err(StakeError::InvalidPayload)
    }
}

#[cfg(test)]
mod tests {

    use primitives::Address;

    use super::*;
    use crate::{keypair::KeyPair, staking::StakeUpdate};

    #[test]
    fn should_create_new_stake() {
        let keypair = KeyPair::random();
        let sk = keypair.miner_kp.0.clone();
        let pk = keypair.miner_kp.1.clone();
        let amount = StakeUpdate::Add(10_000u128);
        let from = Address::new(pk.clone());

        let stake = Stake::new(amount, sk, pk, from, None);

        assert!(stake.is_some());
    }

    #[test]
    fn should_add_certificate_to_stake() {
        let keypair = KeyPair::random();
        let sk = keypair.miner_kp.0.clone();
        let pk = keypair.miner_kp.1.clone();
        let amount = StakeUpdate::Add(10_000u128);
        let from = Address::new(pk.clone());

        let mut stake = Stake::new(amount, sk, pk, from, None).unwrap();

        stake.certify((vec![0u8; 96], vec![0u8; 32])).unwrap();

        assert!(stake.get_certificate().is_some());
    }

    #[test]
    fn should_verify_signature() {
        let keypair = KeyPair::random();
        let sk = keypair.miner_kp.0.clone();
        let pk = keypair.miner_kp.1.clone();
        let amount = StakeUpdate::Add(10_000u128);
        let from = Address::new(pk.clone());

        let stake = Stake::new(amount, sk, pk, from, None).unwrap();

        assert!(stake.verify().is_ok())
    }

    #[test]
    fn from_and_to_should_be_the_same() {
        let keypair = KeyPair::random();
        let sk = keypair.miner_kp.0.clone();
        let pk = keypair.miner_kp.1.clone();
        let amount = StakeUpdate::Add(10_000u128);
        let from = Address::new(pk.clone());

        let stake = Stake::new(amount, sk, pk, from, None).unwrap();

        assert_eq!(stake.get_sender(), stake.get_receiver())
    }

    #[test]
    fn from_and_to_should_be_the_different() {
        let receiver_kp = KeyPair::random();
        let receiver_address = Address::new(receiver_kp.miner_kp.1.clone());
        let keypair = KeyPair::random();
        let sk = keypair.miner_kp.0.clone();
        let pk = keypair.miner_kp.1.clone();
        let amount = StakeUpdate::Add(10_000u128);
        let from = Address::new(pk.clone());

        let stake = Stake::new(amount, sk, pk, from, Some(receiver_address)).unwrap();

        assert_ne!(stake.get_sender(), stake.get_receiver())
    }

    #[test]
    fn should_not_add_certificate_to_stake() {
        let keypair = KeyPair::random();
        let sk = keypair.miner_kp.0.clone();
        let pk = keypair.miner_kp.1.clone();
        let amount = StakeUpdate::Add(10_000u128);
        let from = Address::new(pk.clone());

        let mut stake = Stake::new(amount, sk, pk, from, None).unwrap();

        let result = stake.certify((vec![0u8; 32], vec![0u8; 32]));

        assert!(result.is_err());
        assert!(stake.get_certificate().is_none());
    }
}
