use crate::pool::Pool;
use crate::{block::Block, claim::Claim, txn::Txn};
use ritelinked::LinkedHashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
/// The State of all accounts. This is used to track balances
/// this is also used to track the state of the network in general
/// along with the ClaimState and RewardState. Will need to adjust
/// this to account for smart contracts at some point in the future.
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct AccountState {
    // Map of account address to public key
    pub accounts_pk: LinkedHashMap<String, String>, // K: address, V: pubkey
    pub claim_counter: LinkedHashMap<String, u128>, // K: pubkey, V: number of claims currently owned.
    pub peer_tracker: LinkedHashMap<String, String>, // K: peer_id, V: pubkey.
    pub txn_pool: Pool<String, Txn>,
    pub claim_pool: Pool<String, Claim>,
    pub last_block: Option<Block>,
}

/// The state of all accounts in the network. This is one of the 3 core state objects
/// which ensures that the network maintains consensus amongst nodes. The account state
/// records accounts, along with their native token (VRRB) balances and smart contract
/// token balances. Also contains all pending and confirmed transactions. Pending
/// transactions are set into the pending vector and the confirmed transactions
/// are set in the mineable vector.
impl AccountState {
    pub fn start(txn_pool: Pool<String, Txn>, claim_pool: Pool<String, Claim>) -> AccountState {
        AccountState {
            accounts_pk: LinkedHashMap::new(),
            claim_counter: LinkedHashMap::new(),
            peer_tracker: LinkedHashMap::new(),
            txn_pool,
            claim_pool,
            last_block: None,
        }
    }

    pub fn pending_credits(&self, address: String) -> Option<u128> {
        let mut receipts = self.txn_pool.pending.clone();
        receipts.retain(|_, v| v.receiver_address == address);

        if receipts.is_empty() {
            return None;
        } else {
            let amounts: Vec<u128> = receipts.iter().map(|(_, v)| return v.txn_amount).collect();
            let pending_credits = amounts.iter().sum();
            Some(pending_credits)
        }
    }

    pub fn pending_debits(&self, address: String) -> Option<u128> {
        let mut receipts = self.txn_pool.pending.clone();
        receipts.retain(|_, v| v.sender_address == address);

        if receipts.is_empty() {
            return None;
        } else {
            let amounts: Vec<u128> = receipts.iter().map(|(_, v)| return v.txn_amount).collect();
            let pending_debits = amounts.iter().sum();
            Some(pending_debits)
        }
    }

    pub fn pending_balance(&self, address: String) -> Option<(u128, u128)> {
        let pending_credits = if let Some(amount) = self.pending_credits(address.clone()) {
            amount
        } else {
            0
        };

        let pending_debits = if let Some(amount) = self.pending_debits(address.clone()) {
            amount
        } else {
            0
        };

        Some((pending_credits, pending_debits))
    }

    pub fn add_miner_to_claim_counter(&mut self, pubkey: String) {
        self.claim_counter.entry(pubkey).or_insert(0);
    }

    pub fn add_miner_to_peer_tracker(&mut self, sender_id: String, pubkey: String) {
        self.peer_tracker.entry(sender_id).or_insert(pubkey);
    }

    pub fn remove_miner_from_claim_counter(&mut self, peer_id: String) {
        if let Some(pubkey) = self.peer_tracker.get(&peer_id) {
            self.claim_counter.remove(&pubkey.clone());
        }
    }

    pub fn prune_miners(&mut self, connected_peers: HashSet<String>, local_pubkey: &String) {
        
        self.peer_tracker.retain(|k, _| connected_peers.contains(k));
        
        let connected_peer_pubkeys = self
            .peer_tracker
            .iter()
            .map(|(k, v)| return (v.clone(), k.clone()))
            .collect::<LinkedHashMap<String, String>>();

        self.claim_counter
            .retain(|k, _| {
                connected_peer_pubkeys.contains_key(k) || k == local_pubkey
            }
        );
    }

    pub fn update_claims_owned(&mut self, pubkey: &String, n_claims: u128) {
        if let Some(entry) = self.claim_counter.get_mut(&pubkey.clone()) {
            *entry += n_claims
        } else {
            self.claim_counter.entry(pubkey.clone()).or_insert(n_claims);
        }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_string().as_bytes().to_vec()
    }

    pub fn from_bytes(data: &[u8]) -> AccountState {
        let mut buffer: Vec<u8> = vec![];
        data.iter().for_each(|x| buffer.push(*x));
        let to_string = String::from_utf8(buffer).unwrap();
        AccountState::from_string(&to_string)
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn from_string(string: &String) -> AccountState {
        serde_json::from_str::<AccountState>(string).unwrap()
    }
}

impl Clone for AccountState {
    fn clone(&self) -> Self {
        AccountState {
            accounts_pk: self.accounts_pk.clone(),
            claim_counter: self.claim_counter.clone(),
            peer_tracker: self.peer_tracker.clone(),
            txn_pool: self.txn_pool.clone(),
            claim_pool: self.claim_pool.clone(),
            last_block: self.last_block.clone(),
        }
    }
}
