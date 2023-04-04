use primitives::Address;
use serde::{Deserialize, Serialize};
/// a Module for creating, maintaining, and using a claim in the fair,
/// computationally inexpensive, collission proof, fully decentralized, fully
/// permissionless Proof of Claim Miner Election algorithm
use serde_json;
use sha2::{Sha256, Digest};
use ethereum_types::U256;

use crate::{ownable::Ownable, verifiable::Verifiable, keypair::Keypair};

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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Claim {
    pub public_key: String,
    pub address: String,
    pub hash: U256,
    pub eligible: bool,
}

impl Claim {
    /// Creates a new claim from a public key, address and nonce.
    // TODO: Default nonce to 0
    pub fn new(public_key: String, address: String) -> Claim {
        // Calculate the number of times the pubkey should be hashed to generate the
        // claim hash
        // sequentially hash the public key the correct number of times
        // for the given nonce.
        let mut hasher = Sha256::new();
        hasher.update(public_key.clone());

        let result = hasher.finalize();
        let hash = U256::from_big_endian(&result[..]);
        Claim {
            public_key,
            address,
            hash,
            // Consider setting to false by default 
            // and having it be set to true when harvester 
            // collects threshold of votes on its validity
            eligible: true,
        }
    }

    pub fn get_ballot_info(&self) -> (U256, Claim) {
        (self.hash, self.clone())
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

    /// Calculates the claims pointer sum
    // TODO: Rename to `get_pointer_sum` to better represent the purpose of the
    // function. This can be made significantly faster (if necessary to scale
    // network) by concurrently calculating the index position of each matched
    // character, and summing the total at the end after every match position
    // has been discovered, or returning None if we can't match a character.
    #[deprecated(note = "Please use get_election_result")]
    pub fn get_pointer(&self, block_seed: u128) -> Option<u128> {
        
        // get the hexadecimal format of the block seed
        let block_seed_hex = format!("{block_seed:x}");
        // Get the length of the hexadecimal representation of the block seed
        // for later use
        let block_seed_string_len = block_seed_hex.chars().count();
        // declare an empty mutable vector to stash pointers into.
        let mut pointers = vec![];
        // iterate through (and enumerate for index position) the characters
        // of the block seed.
        let mut hash_bytes = [0u8; 32];
        self.hash.to_big_endian(&mut hash_bytes);
        let hash_string = hex::encode(hash_bytes);

        block_seed_hex.chars().enumerate().for_each(|(idx, c)| {
            // Check if the character is in the claim hash, and save the index position into
            // a variable `n`.
            let res = hash_string.find(c);
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

    fn valid(
        &self,
        item: &Self::Item,
        dependancies: &Self::Dependencies,
    ) -> Result<bool, InvalidClaimError> {
        let _ = item;
        let _ = dependancies;
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

impl From<Keypair> for Claim {
    fn from(item: Keypair) -> Claim {
        Claim::new(
           item.miner_kp.1.to_string(),
           Address::new(item.miner_kp.1).to_string(),
        )
    }
}
