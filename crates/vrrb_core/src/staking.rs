use hbbft::crypto::Signature as Certificate;
use primitives::{Address, Signature};
use secp256k1::Message;
use serde::{Deserialize, Serialize};
use utils::hash_data;

use crate::keypair::{MinerPk, MinerSk};

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
    ///     keypair::{KeyPair, MinerPk, MinerSk},
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
        if let Ok(message) = Message::from_slice(&payload.to_vec()) {
            let signature = sk.sign_ecdsa(message);

            return Some(Stake {
                pubkey: pk.clone(),
                from,
                to,
                amount,
                timestamp,
                signature,
                certificate: None,
            });
        }

        return None;
    }

    /// returns the Stake public key which is used to verify
    /// the signature of the Stake transaction
    pub fn get_pubkey(&self) -> MinerPk {
        self.pubkey.clone()
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

        return self.from.clone();
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
        self.signature.clone()
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
    pub fn certify(&mut self, certificate: Certificate) {
        self.certificate = Some(certificate);
    }
}
