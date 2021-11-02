use crate::block::Block;
use crate::claim::Claim;
use crate::header::BlockHeader;
use crate::pool::{Pool, PoolKind};
use crate::reward::RewardState;
use crate::state::NetworkState;
use crate::txn::Txn;
use crate::validator::TxnValidator;
use crate::verifiable::Verifiable;
use ritelinked::LinkedHashMap;
use serde::{Deserialize, Serialize};
use sha256::digest_bytes;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

pub const VALIDATOR_THRESHOLD: f64 = 0.60;
pub const NANO: u128 = 1;
pub const MICRO: u128 = NANO * 1000;
pub const MILLI: u128 = MICRO * 1000;
pub const SECOND: u128 = MILLI * 1000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MinerStatus {
    Mining,
    Waiting,
    Processing,
}

#[derive(Debug)]
pub struct NoLowestPointerError(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Miner {
    pub claim: Claim,
    pub mining: bool,
    pub claim_map: LinkedHashMap<String, Claim>,
    pub txn_pool: Pool<String, Txn>,
    pub claim_pool: Pool<String, Claim>,
    pub last_block: Option<Block>,
    pub reward_state: RewardState,
    pub network_state: NetworkState,
    pub neighbors: Option<Vec<BlockHeader>>,
    pub current_nonce_timer: u128,
    pub n_miners: u128,
    pub init: bool,
    pub abandoned_claim_counter: LinkedHashMap<String, Claim>,
    pub abandoned_claim: Option<Claim>,
    secret_key: String,
}

impl Miner {
    pub fn start(
        secret_key: String,
        pubkey: String,
        address: String,
        reward_state: RewardState,
        network_state: NetworkState,
        n_miners: u128,
    ) -> Miner {
        let miner = Miner {
            claim: Claim::new(pubkey.clone(), address, 1),
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
        };

        miner
    }

    pub fn get_lowest_pointer(&mut self, nonce: u128) -> Option<(String, u128)> {
        let claim_map = self.claim_map.clone();
        let mut pointers = claim_map
            .iter()
            .map(|(_, claim)| return (claim.clone().hash, claim.clone().get_pointer(nonce)))
            .collect::<Vec<_>>();

        pointers.retain(|(_, v)| !v.is_none());

        let mut raw_pointers = pointers
            .iter()
            .map(|(k, v)| {
                return (k.clone(), v.unwrap());
            })
            .collect::<Vec<_>>();

        if let Some(min) = raw_pointers.clone().iter().min_by_key(|(_, v)| v) {
            raw_pointers.retain(|(_, v)| *v == min.1);
            Some(raw_pointers[0].clone())
        } else {
            None
        }
    }

    pub fn check_my_claim(&mut self, nonce: u128) -> Result<bool, Box<dyn Error>> {
        if let Some((hash, _)) = self.clone().get_lowest_pointer(nonce) {
            return Ok(hash == self.clone().claim.hash);
        } else {
            Err(
                Box::new(
                    NoLowestPointerError("There is no valid pointer, all claims in claim map must increment their nonce by 1".to_string())
                )
            )
        }
    }

    pub fn genesis(&mut self) -> Option<Block> {
        self.claim_map
            .insert(self.claim.pubkey.clone(), self.claim.clone());
        Block::genesis(
            &self.reward_state.clone(),
            self.claim.clone(),
            self.secret_key.clone(),
        )
    }

    pub fn mine(&mut self) -> Option<Block> {
        let claim_map_hash =
            digest_bytes(serde_json::to_string(&self.claim_map).unwrap().as_bytes());
        if let Some(last_block) = self.last_block.clone() {
            return Block::mine(
                self.clone().claim,
                last_block.clone(),
                self.clone().txn_pool.confirmed.clone(),
                self.clone().claim_pool.confirmed.clone(),
                Some(claim_map_hash),
                &self.clone().reward_state.clone(),
                &self.clone().network_state.clone(),
                self.clone().neighbors.clone(),
                self.abandoned_claim.clone(),
                self.secret_key.clone(),
            );
        }

        None
    }

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

    pub fn process_txn(&mut self, mut txn: Txn) -> TxnValidator {
        if let Some(_txn) = self.txn_pool.confirmed.get(&txn.txn_id) {
            // Nothing really to do here
        } else if let Some(txn) = self.txn_pool.pending.get(&txn.txn_id) {
            // add validator if you have not validated already
            if let None = txn.validators.clone().get(&self.claim.pubkey) {
                let mut txn = txn.clone();
                txn.validators.insert(
                    self.claim.pubkey.clone(),
                    txn.valid_txn(&self.network_state, &self.txn_pool),
                );
                self.txn_pool
                    .pending
                    .insert(txn.txn_id.clone(), txn.clone());
            }
        } else {
            // add validator
            txn.validators.insert(
                self.claim.pubkey.clone(),
                txn.valid_txn(&self.network_state, &self.txn_pool),
            );
            self.txn_pool
                .pending
                .insert(txn.txn_id.clone(), txn.clone());
        }

        return TxnValidator::new(
            self.claim.pubkey.clone(),
            txn.clone(),
            &self.network_state,
            &self.txn_pool,
        );
    }

    pub fn process_txn_validator(&mut self, txn_validator: TxnValidator) {
        if let Some(_txn) = self.txn_pool.confirmed.get(&txn_validator.txn.txn_id) {
        } else if let Some(txn) = self.txn_pool.pending.get_mut(&txn_validator.txn.txn_id) {
            txn.validators
                .entry(txn_validator.pubkey)
                .or_insert(txn_validator.vote);
        } else {
            let mut txn = txn_validator.txn.clone();
            txn.validators
                .insert(txn_validator.pubkey, txn_validator.vote);
            self.txn_pool.pending.insert(txn.txn_id.clone(), txn);
        }
    }

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
            let slash_claims = validators
                .iter()
                .map(|(k, _)| return k.to_string())
                .collect::<Vec<_>>();
            return Some(slash_claims);
        } else {
            return None;
        }
    }

    pub fn slash_claim(&mut self, pubkey: String) {
        if let Some(claim) = self.claim_map.get_mut(&pubkey) {
            claim.eligible = false;
        }
    }

    pub fn check_time_elapsed(&self) -> u128 {
        let timestamp = self.get_timestamp();
        if let Some(time) = timestamp.checked_sub(self.current_nonce_timer) {
            time / SECOND
        } else {
            0u128
        }
    }

    pub fn get_timestamp(&self) -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    }

    pub fn abandoned_claim(&mut self, hash: String) {
        self.claim_map.retain(|_, v| v.hash != hash);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        self.current_nonce_timer = timestamp;
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_string().as_bytes().to_vec()
    }

    pub fn from_bytes(data: &[u8]) -> Miner {
        serde_json::from_slice(data).unwrap()
    }

    pub fn from_string(data: &str) -> Miner {
        serde_json::from_str(data).unwrap()
    }

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

impl fmt::Display for NoLowestPointerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for NoLowestPointerError {
    fn description(&self) -> &str {
        &self.0
    }
}
