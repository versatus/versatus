//FEATURE TAG(S): Block Structure, VRF for Next Block Seed, Rewards
use std::{
    collections::HashMap,
    error::Error,
    fmt,
    time::{SystemTime, UNIX_EPOCH},
};

/// This module is for the creation and operation of a mining unit within a node
/// in the network The miner is the primary way that data replication across all
/// nodes occur The mining of blocks can be thought of as incremental
/// checkpoints in the state.
use block::block::Block;
use block::header::BlockHeader;
use claim::claim::Claim;
use noncing::nonceable::Nonceable;
use pool::pool::{Pool, PoolKind};
use primitives::types::Epoch;
use reward::reward::RewardState;
use ritelinked::LinkedHashMap;
use serde::{Deserialize, Serialize};
use sha256::digest;
use state::state::NetworkState;
use txn::txn::Txn;
pub const VALIDATOR_THRESHOLD: f64 = 0.60;
pub const NANO: u128 = 1;
pub const MICRO: u128 = NANO * 1000;
pub const MILLI: u128 = MICRO * 1000;
pub const SECOND: u128 = MILLI * 1000;

// TODO: Consider moving that to genesis_config.yaml
const GENESIS_ALLOWED_MINERS: [&str; 2] = [
    "82104DeE06aa223eC9574a8b2De4fB440630c300",
    "F4ccb23f9A2b10b165965e2a4555EC25615c29BE",
];

/// A basic enum to inform the system whether the current
/// status of the local mining unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MinerStatus {
    Mining,
    Waiting,
    Processing,
}

/// A Basic error type to propagate in the event that there is no
/// valid miner uner the proof of claim algorithm
#[derive(Debug)]
pub struct NoLowestPointerError(String);

/// The miner struct contains all the data and methods needed to operate a
/// mining unit and participate in the data replication process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Miner {
    /// The miner must have a unique claim. This allows them to be included
    /// as a potential miner, and be elected as a miner in the event their
    /// claim returns the lowest pointer sum for a given block seed.
    pub claim: Claim,
    /// A simple status boolean to inform the local system whether the local
    /// mining unit is mining or dealing with a state update or some other
    /// blocking operation that would prevent them from being able to mine a
    /// block
    //TODO: Replace with `MinerStatus` to allow for custom `impl` for different states
    pub mining: bool,
    /// A map of all the claims in the network
    //TODO: Replace with a left-right custom data structure that will better enable efficient
    //maintenance and calculations.
    pub claim_map: LinkedHashMap<String, Claim>,
    /// A pool of pending transactions and their IDs
    //TODO: Replace with Left-Right Mempool, and relative dependent data to include
    // if a given tx requires inclusion in a block (Non-Simple Value Transfer Tx's)
    pub txn_pool: Pool<String, Txn>,
    /// A pool of claims pending approval and acceptance into the network.
    //TODO: Replace with left-right claim pool for more efficient maintenance,
    // validation and calculation.
    pub claim_pool: Pool<String, Claim>,
    /// The most recent block mined, confirmed and propogated throughout the
    ///
    /// network
    pub last_block: Option<Block>,
    /// The reward state (previous monetary policy), to track which reward
    /// categories are still available for production
    //TODO: Eliminate and replace with provable current reward amount data
    pub reward_state: RewardState,
    /// The current state of the network
    //TODO: Replace with ReadHandle in the Left-Right State Trie
    pub network_state: NetworkState,
    /// Neighbor blocks
    //This can either be eliminated, or can include the 2nd and 3rd place finishers in the pointer
    // sum calculation and their proposed `BlockHeader`
    pub neighbors: Option<Vec<BlockHeader>>,
    //TODO: Eliminate
    pub current_nonce_timer: u128,
    /// The total number of miners in the network
    //TODO: Discuss whether this is needed or not
    pub n_miners: u128,
    /// A simple boolean field to denote whether the miner has been initialized
    ///
    /// or not
    pub init: bool,
    /// An ordered map containing claims that were entitled to mine but took too
    ///
    /// long
    pub abandoned_claim_counter: LinkedHashMap<String, Claim>,
    /// The claim of the most recent entitled miner in the event that they took
    ///
    /// too long to propose a block
    //TODO: Discuss a better way to do this, and need to be able to include more than one claim.
    pub abandoned_claim: Option<Claim>,
    /// The secret key of the miner, used to sign blocks they propose to prove
    /// that the block was indeed proposed by the miner with the claim
    /// entitled to mine the given block at the given block height
    secret_key: String,

    epoch: Epoch,
}

impl Miner {
    /// Returns a miner that can be initialized later
    //TODO: Replace `start` with `new`, since this method does not actually "start"
    //
    // the miner
    pub fn start(
        secret_key: String,
        pubkey: String,
        address: String,
        reward_state: RewardState,
        network_state: NetworkState,
        n_miners: u128,
        epoch: Epoch,
    ) -> Self {
        Self {
            claim: Claim::new(pubkey, address, 1),
            mining: false,
            claim_map: LinkedHashMap::new(),
            txn_pool: Pool::new(PoolKind::Txn),
            claim_pool: Pool::new(PoolKind::Claim),
            last_block: None,
            reward_state,
            network_state,
            neighbors: None,
            current_nonce_timer: 0,
            n_miners,
            init: false,
            abandoned_claim_counter: LinkedHashMap::new(),
            abandoned_claim: None,
            secret_key,
            epoch,
        }
    }

    /// Calculates the pointer sums and returns the lowest for a given block
    ///
    /// seed.
    pub fn get_lowest_pointer(&mut self, block_seed: u128) -> Option<(String, u128)> {
        // Clones the local claim map for use in the algorithm
        let claim_map = self.claim_map.clone();

        // Calculates the pointers for every claim, for the given block seed, in the map
        // and collects them into a vector of tuples containing the claim hash and the
        //
        // pointer sum
        let mut pointers = claim_map
            .iter()
            .map(|(_, claim)| (claim.clone().hash, claim.clone().get_pointer(block_seed)))
            .collect::<Vec<_>>();

        // Retains only the pointers that have Some(value) in the 2nd field of the tuple
        // `.get_pointer(block_seed)` returns an Option<u128>, and can return the None
        // variant in the event that the pointer sum contains integer overflows
        // OR in the event that not ever
        pointers.retain(|(_, v)| !v.is_none());

        // unwraps all the pointer sum values.
        //TODO: make this more efficient, this is a wasted operation
        let mut base_pointers = pointers
            .iter()
            .map(|(k, v)| (k.clone(), v.unwrap()))
            .collect::<Vec<_>>();

        // check if there's a minimum, and return the key of the lowest
        if let Some(min) = base_pointers.clone().iter().min_by_key(|(_, v)| v) {
            base_pointers.retain(|(_, v)| *v == min.1);
            Some(base_pointers[0].clone())
        } else {
            None
        }
    }

    /// Checks if the hash of the claim with the lowest pointer sum is the local
    ///
    /// claim.
    pub fn check_my_claim(&mut self, nonce: u128) -> Result<bool, Box<dyn Error>> {
        if let Some((hash, _)) = self.clone().get_lowest_pointer(nonce) {
            Ok(hash == self.clone().claim.hash)
        } else {
            Err(
                Box::new(
                    NoLowestPointerError("There is no valid pointer, all claims in claim map must increment their nonce by 1".to_string())
                )
            )
        }
    }

    /// Generates a gensis block
    //TODO: Require a specific key to mine the genesis block so that only one node
    // controlled by the organization can mine it.
    pub fn genesis(&mut self) -> Option<Block> {
        if !GENESIS_ALLOWED_MINERS.contains(&&*self.claim.pubkey) {
            return None;
        }
        self.claim_map
            .insert(self.claim.pubkey.clone(), self.claim.clone());
        Block::genesis(
            &self.reward_state.clone(),
            self.claim.clone(),
            self.secret_key.clone(),
        )
    }

    /// Attempts to mine a block
    //TODO: Require more stringent checks to see if the block is able to be mined.
    pub fn mine(&mut self) -> (Option<Block>, u128) {
        let claim_map_hash = digest(serde_json::to_string(&self.claim_map).unwrap().as_bytes());
        if let Some(last_block) = self.last_block.clone() {
            return Block::mine(
                self.clone().claim,
                last_block,
                self.clone().txn_pool.confirmed,
                self.clone().claim_pool.confirmed,
                Some(claim_map_hash),
                &self.clone().reward_state.clone(),
                &self.clone().network_state,
                self.clone().neighbors,
                self.abandoned_claim.clone(),
                self.secret_key.clone(),
                self.epoch,
            );
        }

        (None, 0)
    }

    /// Increases the nonce and calculates the new hash for all claims
    /// This only occurs in the event that no claims return valid pointer sums.
    pub fn nonce_up(&mut self) {
        self.claim.nonce_up();
        let mut new_claim_map = LinkedHashMap::new();
        self.claim_map.clone().iter().for_each(|(pk, claim)| {
            let mut new_claim = claim.clone();
            new_claim.nonce_up();
            new_claim_map.insert(pk.clone(), new_claim.clone());
        });
        self.claim_map = new_claim_map;
    }

    /// Checks if the transaction has been confirmed
    //TODO: Either eliminate and replace, each miner should retain only
    // transactions that have be pre-validated i.e. they should only have the
    // "confirmed side" of the Mempool.
    pub fn check_confirmed(&mut self, txn_id: String) {
        let mut validators = {
            if let Some(txn) = self.txn_pool.pending.get(&txn_id) {
                txn.validators.clone()
            } else {
                HashMap::new()
            }
        };

        validators.retain(|_, v| *v);
        if validators.len() as f64 / (self.claim_map.len() - 1) as f64 > VALIDATOR_THRESHOLD {
            if let Some((k, v)) = self.txn_pool.pending.remove_entry(&txn_id) {
                self.txn_pool.confirmed.insert(k, v);
            }
        }
    }

    /// Checks if the transaction has been rejected
    //TODO: Either eliminate and replace. Each miner should retain only
    // transactions that have bee pre-validated, i.e. they should only have the
    // "confirmed side" of the Mempool
    pub fn check_rejected(&self, txn_id: String) -> Option<Vec<String>> {
        let mut validators = {
            if let Some(txn) = self.txn_pool.pending.get(&txn_id) {
                txn.validators.clone()
            } else {
                HashMap::new()
            }
        };

        let mut rejected = validators.clone();
        rejected.retain(|_, v| !*v);
        validators.retain(|_, v| *v);

        if rejected.len() as f64 / self.claim_map.len() as f64 > 1.0 - VALIDATOR_THRESHOLD {
            let slash_claims = validators.keys().map(|k| k.to_string()).collect::<Vec<_>>();
            Some(slash_claims)
        } else {
            None
        }
    }

    /// Turns a claim into an ineligible claim in the event a miner proposes an
    /// invalid block or tries to spam the network.
    //TODO: Need much stricter penalty, as this miner can still send messages. The
    // transport layer should reject further messages from this node.
    pub fn slash_claim(&mut self, pubkey: String) {
        if let Some(claim) = self.claim_map.get_mut(&pubkey) {
            claim.eligible = false;
        }
    }

    /// Checks how much time has passed since the entitled miner has not
    ///
    /// proposed a block
    pub fn check_time_elapsed(&self) -> u128 {
        let timestamp = self.get_timestamp();
        if let Some(time) = timestamp.checked_sub(self.current_nonce_timer) {
            time / SECOND
        } else {
            0u128
        }
    }

    /// Gets a local current timestamp
    pub fn get_timestamp(&self) -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    }

    /// Abandons the claim of a miner that fails to proppose a block in the
    ///
    /// proper amount of time.
    pub fn abandoned_claim(&mut self, hash: String) {
        self.claim_map.retain(|_, v| v.hash != hash);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        self.current_nonce_timer = timestamp;
    }

    /// Serializes the miner into a string
    // TODO: Consider changing this to `serialize_to_string`
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }

    /// Serializes the miner into a vector of bytes
    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_string().as_bytes().to_vec()
    }

    /// Deserializes a miner from a byte array
    pub fn from_bytes(data: &[u8]) -> Miner {
        serde_json::from_slice(data).unwrap()
    }

    /// Deserializes a miner from a string slice
    pub fn from_string(data: &str) -> Miner {
        serde_json::from_str(data).unwrap()
    }

    /// Returns a vetor of string representations of the field names of a miner
    pub fn get_field_names(&self) -> Vec<String> {
        vec![
            "claim".to_string(),
            "mining".to_string(),
            "claim_map".to_string(),
            "txn_pool".to_string(),
            "claim_pool".to_string(),
            "last_block".to_string(),
            "reward_state".to_string(),
            "network_state".to_string(),
            "neighbors".to_string(),
            "current_nonce_timer".to_string(),
            "n_miners".to_string(),
            "init".to_string(),
            "abandoned_claim_counter".to_string(),
            "abandoned_claim".to_string(),
            "secret_key".to_string(),
        ]
    }
}

/// Required for `NoLowestPointerError` to be able to be used as an Error type
///
/// in the Result enum
impl fmt::Display for NoLowestPointerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Required for `NoLowestPointerError` to be able to be used as an Error type
///
/// in the Result enum
impl Error for NoLowestPointerError {
    fn description(&self) -> &str {
        &self.0
    }
}
