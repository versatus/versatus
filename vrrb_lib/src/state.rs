use crate::pool::Pool;
use crate::txn::Txn;
use crate::{block::Block, claim::Claim, reward::RewardState};
use log::info;
use ritelinked::LinkedHashMap;
use serde::{Deserialize, Serialize};
use sha256::digest_bytes;
use std::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Ledger {
    pub credits: LinkedHashMap<String, u128>,
    pub debits: LinkedHashMap<String, u128>,
    pub claims: LinkedHashMap<String, Claim>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Components {
    pub genesis: Option<Vec<u8>>,
    pub child: Option<Vec<u8>>,
    pub parent: Option<Vec<u8>>,
    pub blockchain: Option<Vec<u8>>,
    pub ledger: Option<Vec<u8>>,
    pub network_state: Option<Vec<u8>>,
    pub archive: Option<Vec<u8>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkState {
    // Path to database
    pub path: String,
    pub ledger: Vec<u8>,
    // hash of the state of credits in the network
    pub credits: Option<String>,
    // hash of the state of debits in the network
    pub debits: Option<String>,
    //reward state of the network
    pub reward_state: RewardState,
    // the last state hash -> sha256 hash of credits, debits & reward state.
    pub state_hash: Option<String>,
}

impl NetworkState {
    pub fn restore(path: &str) -> NetworkState {

        if let Ok(string) = fs::read_to_string(path) {
            NetworkState::from_bytes(&hex::decode(string).unwrap())
        } else {
            let network_state = NetworkState {
                path: path.to_string(),
                ledger: Ledger::new().as_bytes(),
                credits: None,
                debits: None,
                reward_state: RewardState::start(),
                state_hash: None,
            };

            network_state.dump_to_file();
            network_state
        }
    }

    pub fn get_balance(&self, address: &str) -> u128 {
        let credits = self.get_account_credits(address);
        let debits = self.get_account_debits(address);

        if let Some(balance) = credits.checked_sub(debits) {
            return balance;
        } else {
            return 0u128;
        }
    }

    pub fn credit_hash(self, block: &Block) -> String {
        let mut credits = LinkedHashMap::new();

        block.txns.iter().for_each(|(_txn_id, txn)| {
            if let Some(entry) = credits.get_mut(&txn.receiver_address) {
                *entry += txn.clone().txn_amount
            } else {
                credits.insert(txn.clone().receiver_address, txn.clone().txn_amount);
            }
        });

        if let Some(entry) = credits.get_mut(&block.header.block_reward.miner.clone().unwrap()) {
            *entry += block.header.block_reward.amount
        } else {
            credits.insert(
                block.header.block_reward.miner.clone().unwrap(),
                block.header.block_reward.amount,
            );
        }

        if let Some(chs) = self.credits {
            return digest_bytes(format!("{},{:?}", chs, credits).as_bytes());
        } else {
            return digest_bytes(format!("{:?},{:?}", self.credits, credits).as_bytes());
        }
    }

    pub fn debit_hash(self, block: &Block) -> String {
        let mut debits = LinkedHashMap::new();
        block.txns.iter().for_each(|(_txn_id, txn)| {
            if let Some(entry) = debits.get_mut(&txn.sender_address) {
                *entry += txn.clone().txn_amount
            } else {
                debits.insert(txn.clone().sender_address, txn.clone().txn_amount);
            }
        });

        if let Some(dhs) = self.debits {
            return digest_bytes(format!("{},{:?}", dhs, debits).as_bytes());
        } else {
            return digest_bytes(format!("{:?},{:?}", self.debits, debits).as_bytes());
        }
    }

    pub fn hash(&mut self, block: Block) -> String {
        let credit_hash = self.clone().credit_hash(&block);
        let debit_hash = self.clone().debit_hash(&block);
        let reward_state_hash = digest_bytes(format!("{:?}", self.reward_state).as_bytes());
        let payload = format!(
            "{:?},{:?},{:?},{:?}",
            self.state_hash, credit_hash, debit_hash, reward_state_hash
        );
        let new_state_hash = digest_bytes(payload.as_bytes());
        new_state_hash
    }

    pub fn dump(&mut self, block: &Block) {
        let mut ledger = Ledger::from_bytes(&self.ledger);

        block.txns.iter().for_each(|(_txn_id, txn)| {
            if let Some(entry) = ledger.credits.get_mut(&txn.receiver_address) {
                *entry += txn.clone().txn_amount;
            } else {
                ledger
                    .credits
                    .insert(txn.clone().receiver_address, txn.clone().txn_amount);
            }

            if let Some(entry) = ledger.debits.get_mut(&txn.clone().sender_address) {
                *entry += txn.clone().txn_amount;
            } else {
                ledger
                    .debits
                    .insert(txn.clone().sender_address, txn.clone().txn_amount);
            }
        });

        block.claims.iter().for_each(|(k, v)| {
            ledger.claims.insert(k.clone(), v.clone());
        });

        ledger.claims.insert(
            block.header.claim.clone().pubkey,
            block.header.claim.clone(),
        );

        if let Some(entry) = ledger
            .credits
            .get_mut(&block.header.block_reward.miner.clone().unwrap())
        {
            *entry += block.header.block_reward.amount.clone();
        } else {
            ledger.credits.insert(
                block.header.block_reward.miner.clone().unwrap().clone(),
                block.header.block_reward.amount.clone(),
            );
        }

        self.update_reward_state(&block);
        self.update_state_hash(&block);
        self.update_reward_state(&block);
        self.update_credits_and_debits(&block);

        let ledger_hex = hex::encode(ledger.clone().as_bytes());
        if let Err(_) = fs::write(self.path.clone(), ledger_hex) {
            info!("Error writing ledger hex to file");
        };

        self.ledger = ledger.as_bytes();
    }

    pub fn nonce_up(&mut self) {
        let mut new_claim_map = LinkedHashMap::new();
        self.get_claims().clone().iter().for_each(|(pk, claim)| {
            let mut new_claim = claim.clone();
            new_claim.nonce_up();
            new_claim_map.insert(pk.clone(), new_claim.clone());
        });

        let mut ledger = Ledger::from_bytes(&self.ledger);
        ledger.claims = new_claim_map;
        self.ledger = ledger.as_bytes();
    }

    pub fn abandoned_claim(&mut self, hash: String) {
        let mut ledger = Ledger::from_bytes(&self.ledger.clone());
        ledger.claims.retain(|_, v| v.hash != hash);
        self.ledger = ledger.as_bytes();
        self.dump_to_file();
    }

    pub fn restore_ledger(&self) -> Ledger {
        let network_state_hex = fs::read_to_string(self.path.clone()).unwrap();
        let network_state = NetworkState::from_bytes(&hex::decode(network_state_hex).unwrap());
        Ledger::from_bytes(&network_state.ledger.clone())
    }

    pub fn update_credits_and_debits(&mut self, block: &Block) {
        let chs = self.clone().credit_hash(block);
        let dhs = self.clone().debit_hash(block);
        self.credits = Some(chs);
        self.debits = Some(dhs);
    }

    pub fn update_reward_state(&mut self, block: &Block) {
        self.reward_state.update(block.header.block_reward.category);
    }

    pub fn update_state_hash(&mut self, block: &Block) {
        self.state_hash = Some(block.hash.clone());
    }

    pub fn get_credits(&self) -> LinkedHashMap<String, u128> {
        Ledger::from_bytes(&self.ledger).credits.clone()
    }

    pub fn get_debits(&self) -> LinkedHashMap<String, u128> {
        Ledger::from_bytes(&self.ledger).debits.clone()
    }

    pub fn get_claims(&self) -> LinkedHashMap<String, Claim> {
        Ledger::from_bytes(&self.ledger).claims.clone()
    }

    pub fn get_reward_state(&self) -> RewardState {
        self.reward_state
    }

    pub fn get_account_credits(&self, address: &str) -> u128 {
        let credits = self.get_credits();
        if let Some(amount) = credits.get(address) {
            return *amount;
        } else {
            return 0u128;
        }
    }

    pub fn get_account_debits(&self, address: &str) -> u128 {
        let debits = self.get_debits();
        if let Some(amount) = debits.get(address) {
            return *amount;
        } else {
            return 0u128;
        }
    }
    pub fn update_ledger(&mut self, ledger: Ledger) {
        self.ledger = ledger.as_bytes();
    }

    pub fn get_lowest_pointer(&self, nonce: u128) -> Option<(String, u128)> {
        let claim_map = self.get_claims();
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

    pub fn slash_claims(&mut self, bad_validators: Vec<String>) {
        let mut ledger = Ledger::from_bytes(&self.ledger.clone());
        bad_validators.iter().for_each(|k| {
            if let Some(claim) = ledger.claims.get_mut(&k.to_string()) {
                claim.eligible = false;
            }
        });
        self.ledger = ledger.as_bytes();
        self.dump_to_file()
    }

    pub fn pending_balance(
        &self,
        _address: String,
        _txn_pool: &Pool<String, Txn>,
    ) -> Option<(u128, u128)> {
        None
    }

    pub fn dump_to_file(&self) {
        if let Err(_) = fs::write(self.path.clone(), hex::encode(self.as_bytes())) {
            info!("Error dumping ledger to file");
        };
    }

    pub fn credits_as_bytes(credits: &LinkedHashMap<String, u128>) -> Vec<u8> {
        NetworkState::credits_to_string(credits).as_bytes().to_vec()
    }

    pub fn credits_to_string(credits: &LinkedHashMap<String, u128>) -> String {
        serde_json::to_string(credits).unwrap()
    }

    pub fn credits_from_bytes(data: &[u8]) -> LinkedHashMap<String, u128> {
        serde_json::from_slice::<LinkedHashMap<String, u128>>(data).unwrap()
    }

    pub fn debits_as_bytes(debits: &LinkedHashMap<String, u128>) -> Vec<u8> {
        NetworkState::debits_to_string(debits).as_bytes().to_vec()
    }

    pub fn debits_to_string(debits: &LinkedHashMap<String, u128>) -> String {
        serde_json::to_string(debits).unwrap()
    }

    pub fn debits_from_bytes(data: &[u8]) -> LinkedHashMap<String, u128> {
        serde_json::from_slice::<LinkedHashMap<String, u128>>(data).unwrap()
    }

    pub fn claims_as_bytes(claims: &LinkedHashMap<u128, Claim>) -> Vec<u8> {
        NetworkState::claims_to_string(claims).as_bytes().to_vec()
    }

    pub fn claims_to_string(claims: &LinkedHashMap<u128, Claim>) -> String {
        serde_json::to_string(claims).unwrap()
    }

    pub fn claims_from_bytes(data: &[u8]) -> LinkedHashMap<u128, Claim> {
        serde_json::from_slice::<LinkedHashMap<u128, Claim>>(data).unwrap()
    }

    pub fn last_block_from_bytes(data: &[u8]) -> Block {
        serde_json::from_slice::<Block>(data).unwrap()
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_string().as_bytes().to_vec()
    }

    pub fn from_bytes(data: &[u8]) -> NetworkState {
        serde_json::from_slice::<NetworkState>(data).unwrap()
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn from_string(string: &String) -> NetworkState {
        serde_json::from_str::<NetworkState>(&string).unwrap()
    }

    pub fn db_to_ledger(&self) -> Ledger {
        let credits = self.get_credits();
        let debits = self.get_debits();
        let claims = self.get_claims();

        Ledger {
            credits,
            debits,
            claims,
        }
    }
}

impl Ledger {
    pub fn new() -> Ledger {
        Ledger {
            credits: LinkedHashMap::new(),
            debits: LinkedHashMap::new(),
            claims: LinkedHashMap::new(),
        }
    }
    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_string().as_bytes().to_vec()
    }

    pub fn from_bytes(data: &[u8]) -> Ledger {
        serde_json::from_slice::<Ledger>(data).unwrap()
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn from_string(string: &String) -> Ledger {
        serde_json::from_str::<Ledger>(&string).unwrap()
    }
}

impl Components {
    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_string().as_bytes().to_vec()
    }

    pub fn from_bytes(data: &[u8]) -> Components {
        serde_json::from_slice::<Components>(data).unwrap()
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn from_string(string: &String) -> Components {
        serde_json::from_str::<Components>(&string).unwrap()
    }
}

impl Clone for NetworkState {
    fn clone(&self) -> NetworkState {
        NetworkState {
            path: self.path.clone(),
            ledger: self.ledger.clone(),
            credits: self.credits.clone(),
            debits: self.debits.clone(),
            reward_state: self.reward_state.clone(),
            state_hash: self.state_hash.clone(),
        }
    }
}
