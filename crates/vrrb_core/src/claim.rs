use std::collections::HashMap;

use serde::{Deserialize, Serialize};
/// a Module for creating, maintaining, and using a claim in the fair,
/// computationally inexpensive, collission proof, fully decentralized, fully
/// permissionless Proof of Claim Miner Election algorithm
use serde_json;
use sha256::digest;

use crate::{nonceable::Nonceable, ownable::Ownable, verifiable::Verifiable};

/// A custom error type for invalid claims that are used/attempted to be used
/// in the mining of a block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidClaimError {
    details: String,
}

/// The claim object that stores the key information used to mine blocks,
/// calculate whether or not you are an entitled miner, and to share with
/// network
// TODO: Add staking to the claim.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Claim {
    pub public_key: String,
    pub address: String,
    pub hash: String,
    pub nonce: u128,
    pub eligible: bool,
}

impl Claim {
    /// Creates a new claim from a public key, address and nonce.
    // TODO: Default nonce to 0
    pub fn new(public_key: String, address: String, claim_nonce: u128) -> Claim {
        // Calculate the number of times the pubkey should be hashed to generate the
        // claim hash
        let iters = if let Some(n) = claim_nonce.checked_mul(10) {
            n
        } else {
            claim_nonce
        };

        let mut hash = public_key.clone();
        // sequentially hash the public key the correct number of times
        // for the given nonce.
        (0..iters).for_each(|_| {
            hash = digest(hash.as_bytes());
        });

        Claim {
            public_key: public_key.clone(),
            address,
            hash,
            nonce: claim_nonce,
            eligible: true,
        }
    }

    /// Calculates the claims pointer sum
    // TODO: Rename to `get_pointer_sum` to better represent the purpose of the
    // function. This can be made significantly faster (if necessary to scale
    // network) by concurrently calculating the index position of each matched
    // character, and summing the total at the end after every match position
    // has been discovered, or returning None if we can't match a character.
    pub fn get_pointer(&self, block_seed: u128) -> Option<u128> {
        // get the hexadecimal format of the block seed
        // TODO: Make the block seed hexadecimal to begin with in the `Block` itself
        // No reason for miners to have to do this conversion.
        let block_seed_hex = format!("{block_seed:x}");
        // Get the length of the hexadecimal representation of the block seed
        // for later use
        let block_seed_string_len = block_seed_hex.chars().count();
        // declare an empty mutable vector to stash pointers into.
        let mut pointers = vec![];
        // iterate through (and enumerate for index position) the characters
        // of the block seed.
        block_seed_hex.chars().enumerate().for_each(|(idx, c)| {
            // Check if the character is in the claim hash, and save the index position into
            // a variable `n`.
            let res = self.hash.find(c);
            if let Some(n) = res {
                // convert `n` to a u128 and calculate an integer overflow safe
                // exponential of the `n` to the power of idx
                let n = n as u128;
                let n = n.checked_pow(idx as u32);
                // If there is no integer overflow (which there never should be)
                // add it to the buffer.
                if let Some(n) = n {
                    pointers.push(n);
                }
            }
        });

        // If the length of the pointer buffer is the same length
        // as the block seed hex string then calculate the sum as a u128
        // TODO: use integer overflow safe sum, though there should never be
        // an integer overflow with the pointer sum being a u128, it is better
        // to be safe than sorry.
        if pointers.len() == block_seed_string_len {
            let pointer: u128 = pointers.iter().sum();
            Some(pointer)
        } else {
            // If the length of the pointer buffer is not the same length
            // as the block seed hex string, return None, not every character was
            // matched.
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
            "nonce".to_string(),
            "eligible".to_string(),
        ]
    }
}

/// Implements Verifiable trait on Claim
// TODO: Need to actually implement the valid method
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
    fn get_pubkey(&self) -> String {
        self.public_key.clone()
    }
}

/// Implements the Nonceble train on the `Claim`
impl Nonceable for Claim {
    fn nonceable(&self) -> bool {
        true
    }

    fn nonce_up(&mut self) {
        self.nonce += 1;
        let iters = if let Some(n) = self.nonce.checked_mul(10) {
            n
        } else {
            self.nonce
        };

        let mut hash = self.public_key.clone();
        (0..iters).for_each(|_| {
            hash = digest(hash.as_bytes());
        });

        self.hash = hash;
    }
}
