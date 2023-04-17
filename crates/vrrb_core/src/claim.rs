use ethereum_types::U256;
use primitives::{Address, PublicKey};
use serde::{Deserialize, Serialize};
/// a Module for creating, maintaining, and using a claim in the fair,
/// computationally inexpensive, collission proof, fully decentralized, fully
/// permissionless Proof of Claim Miner Election algorithm
use serde_json;
use sha2::{Digest, Sha256};

use crate::{
    keypair::Keypair,
    ownable::Ownable,
    staking::{Stake, StakeUpdate},
    verifiable::Verifiable,
};

/// A custom error type for invalid claims that are used/attempted to be used
/// in the mining of a block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidClaimError {
    details: String,
}

/// The claim object that stores the key information used to mine blocks,
/// calculate whether or not you are an entitled miner, and to share with
/// network
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Claim {
    pub public_key: PublicKey,
    pub address: Address,
    pub hash: U256,
    pub eligible: bool,
    stake: u128,
    stake_txns: Vec<Stake>,
}

impl Claim {
    /// Creates a new claim from a public key, address and nonce.
    pub fn new(public_key: PublicKey, address: Address) -> Claim {
        let mut hasher = Sha256::new();
        hasher.update(public_key.to_string().clone());
        let result = hasher.finalize();
        let hash = U256::from_big_endian(&result[..]);
        Claim {
            public_key,
            address,
            hash,
            eligible: true,
            stake: 0,
            stake_txns: vec![],
        }
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
    pub fn update_stake(&mut self, stake_txn: Stake) {
        if let Some(_) = stake_txn.get_certificate() {
            self.stake_txns.push(stake_txn.clone());
            self.stake = self.check_stake_utxo();
        }
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
        return value - slash as u128;
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
}

/// Implements Verifiable trait on Claim
impl Verifiable for Claim {
    type Dependencies = (Option<Vec<u8>>, Option<Vec<u8>>);
    type Error = InvalidClaimError;
    type Item = Option<Vec<u8>>;

    fn verifiable(&self) -> bool {
        true
    }

    #[allow(unused_variables)]
    fn valid(
        &self,
        item: &Self::Item,
        dependancies: &Self::Dependencies,
    ) -> Result<bool, InvalidClaimError> {
        Ok(true)
    }
}

/// Implements the Ownable trait on a claim
// TODO: Add more methods that make sense for Ownable to Ownable
impl Ownable for Claim {
    type Pubkey = PublicKey;

    fn get_pubkey(&self) -> PublicKey {
        self.public_key.clone()
    }
}

impl From<Keypair> for Claim {
    fn from(item: Keypair) -> Claim {
        Claim::new(item.miner_kp.1.clone(), Address::new(item.miner_kp.1))
    }
}
