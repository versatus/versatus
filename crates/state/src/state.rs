//FEATURE TAG(S): Left-Right Database, Left-Right State Trie
use std::fs;

use reward::reward::Reward;
use ritelinked::LinkedHashMap;
use serde::{Deserialize, Serialize};
use sha256::digest;
use telemetry::info;
/// This module contains the Network State struct (which will be replaced with
/// the Left-Right State Trie)
use vrrb_core::accountable::Accountable;
use vrrb_core::{claim::Claim, ledger::Ledger, nonceable::Nonceable, ownable::Ownable};

use crate::types::{
    CreditsHash,
    CreditsRoot,
    DebitsHash,
    DebitsRoot,
    LedgerBytes,
    StateHash,
    StatePath,
    StateRewardState,
    StateRoot,
};

/// The Network State struct, contains basic information required to determine
/// the current state of the network.
//TODO: Replace `ledger`, `credits`, `debits`, with LR State Trie
//TODO: Replace `state_hash` with LR State Trie Root.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct NetworkState {
    // Path to database
    pub path: StatePath,
    // ledger.as_bytes()
    pub ledger: LedgerBytes,
    // hash of the state of credits in the network
    pub credits: CreditsRoot,
    // hash of the state of debits in the network
    pub debits: DebitsRoot,
    //reward state of the network
    pub reward: StateRewardState,
    // the last state hash -> sha256 hash of credits, debits & reward state.
    pub state_hash: StateRoot,
}

impl<'de> NetworkState {
    /// Restores the network state from a serialized hex string representation
    /// and returns a proper struct
    pub fn restore(path: &str) -> NetworkState {
        let hex_string = {
            if let Ok(string) = fs::read_to_string(path) {
                string
            } else {
                String::new()
            }
        };

        let bytes = hex::decode(hex_string);
        if let Ok(state_bytes) = bytes {
            if let Ok(network_state) = NetworkState::from_bytes(state_bytes) {
                network_state.dump_to_file();
                return network_state;
            }
        }

        // TODO: decode db from bytes and feed it to network_state

        let network_state = NetworkState {
            path: path.into(),
            ledger: vec![],
            credits: None,
            debits: None,
            reward: None,
            state_hash: None,
        };

        network_state.dump_to_file();
        network_state
    }

    /// Dumps a new ledger (serialized in a vector of bytes) to a file.
    pub fn set_ledger(&mut self, ledger_bytes: LedgerBytes) {
        self.ledger = ledger_bytes;
        self.dump_to_file();
    }

    /// Sets a new `RewardState` to the `reward_state` filed in the
    /// `NetworkState` and dumps the resulting new state to the file
    pub fn set_reward(&mut self, reward: Reward) {
        self.reward = Some(reward);
        self.dump_to_file();
    }

    /// Gets the balance of a given address from the network state
    pub fn get_balance(&self, address: &str) -> u128 {
        let credits = self.get_account_credits(address);
        let debits = self.get_account_debits(address);

        if let Some(balance) = credits.checked_sub(debits) {
            balance
        } else {
            0u128
        }
    }

    /// Calculates a new/updated `CreditsHash`
    pub fn credit_hash<A: Accountable, R: Accountable>(
        self,
        txns: &LinkedHashMap<String, A>,
        reward: R,
    ) -> CreditsHash {
        let mut credits = LinkedHashMap::new();

        txns.iter().for_each(|(_txn_id, txn)| {
            if let Some(entry) = credits.get_mut(&txn.receivable()) {
                *entry += txn.get_amount()
            } else {
                credits.insert(txn.receivable(), txn.get_amount());
            }
        });

        if let Some(entry) = credits.get_mut(&reward.receivable()) {
            *entry += reward.get_amount()
        } else {
            credits.insert(reward.receivable(), reward.get_amount());
        }

        if let Some(chs) = self.credits {
            return digest(format!("{},{:?}", chs, credits).as_bytes());
        } else {
            return digest(format!("{:?},{:?}", self.credits, credits).as_bytes());
        }
    }

    /// Calculates a new/updated `DebitsHash`
    pub fn debit_hash<A: Accountable>(self, txns: &LinkedHashMap<String, A>) -> DebitsHash {
        let mut debits = LinkedHashMap::new();
        txns.iter().for_each(|(_txn_id, txn)| {
            if let Some(payable) = txn.payable() {
                if let Some(entry) = debits.get_mut(&payable) {
                    *entry += txn.get_amount()
                } else {
                    debits.insert(payable.clone(), txn.get_amount());
                }
            }
        });

        if let Some(dhs) = self.debits {
            return digest(format!("{},{:?}", dhs, debits).as_bytes());
        } else {
            return digest(format!("{:?},{:?}", self.debits, debits).as_bytes());
        }
    }

    /// Hashes the current credits, debits and reward state and returns a new
    /// `StateHash`
    #[deprecated(note = "replaced by lr-trie and trie roots")]
    pub fn hash<A: Accountable, R: Accountable>(
        &mut self,
        txns: &LinkedHashMap<String, A>,
        reward: R,
    ) -> StateHash {
        let credit_hash = self.clone().credit_hash(txns, reward);
        let debit_hash = self.clone().debit_hash(txns);
        let reward_state_hash = digest(format!("{:?}", self.reward).as_bytes());
        let payload = format!(
            "{:?},{:?},{:?},{:?}",
            self.state_hash, credit_hash, debit_hash, reward_state_hash
        );
        let new_state_hash = digest(payload.as_bytes());
        new_state_hash
    }

    /// Updates the ledger and dumps it to a file
    pub fn dump<A: Accountable>(
        &mut self,
        txns: &LinkedHashMap<String, A>,
        reward: Reward,
        claims: &LinkedHashMap<String, Claim>,
        miner_claim: Claim,
        hash: &String,
    ) {
        let mut ledger = Ledger::<Claim>::from_bytes(self.ledger.clone());

        txns.iter().for_each(|(_txn_id, txn)| {
            if let Some(entry) = ledger.credits.get_mut(&txn.receivable()) {
                *entry += txn.get_amount();
            } else {
                ledger.credits.insert(txn.receivable(), txn.get_amount());
            }

            if let Some(payable) = txn.payable() {
                if let Some(entry) = ledger.debits.get_mut(&payable) {
                    *entry += txn.get_amount();
                } else {
                    ledger.debits.insert(payable, txn.get_amount());
                }
            }
        });

        claims.iter().for_each(|(k, v)| {
            ledger.claims.insert(k.clone(), v.clone());
        });

        ledger.claims.insert(miner_claim.get_pubkey(), miner_claim);

        if let Some(entry) = ledger.credits.get_mut(&reward.receivable()) {
            *entry += reward.get_amount();
        } else {
            ledger
                .credits
                .insert(reward.receivable(), reward.get_amount());
        }

        self.update_state_hash(hash);
        self.update_credits_and_debits(txns, reward);

        let ledger_hex = hex::encode(ledger.clone().as_bytes());
        if fs::write(self.path.clone(), ledger_hex).is_err() {
            info!("Error writing ledger hex to file");
        };

        self.ledger = ledger.as_bytes();
    }

    // TODO: refactor to handle NetworkState nonce_up() a different way, since
    // closure requires explicit types and explicit type specification would
    // lead to cyclical dependencies.
    /// nonces all claims in the ledger up one.
    pub fn nonce_up(&mut self) {
        let mut new_claim_map: LinkedHashMap<String, Claim> = LinkedHashMap::new();
        let claims: LinkedHashMap<String, Claim> = self.get_claims();
        claims.iter().for_each(|(pk, claim)| {
            let mut new_claim = claim.clone();
            new_claim.nonce_up();
            new_claim_map.insert(pk.clone(), new_claim.clone());
        });

        let mut ledger = Ledger::from_bytes(self.ledger.clone());
        ledger.claims = new_claim_map;
        self.ledger = ledger.as_bytes();
    }

    /// Abandons a claim in the Ledger
    pub fn abandoned_claim(&mut self, hash: String) {
        let mut ledger: Ledger<Claim> = Ledger::from_bytes(self.ledger.clone());
        ledger.claims.retain(|_, v| v.hash != hash);
        self.ledger = ledger.as_bytes();
        self.dump_to_file();
    }

    /// Restors the ledger from a hex string representation stored in a file to
    /// a proper ledger
    pub fn restore_ledger(&self) -> Ledger<Claim> {
        let network_state_hex = fs::read_to_string(self.path.clone()).unwrap();
        let bytes = hex::decode(network_state_hex);
        if let Ok(state_bytes) = bytes {
            if let Ok(network_state) = NetworkState::from_bytes(state_bytes) {
                Ledger::from_bytes(network_state.ledger)
            } else {
                Ledger::new()
            }
        } else {
            Ledger::new()
        }
    }

    /// Updates the credit and debit hashes in the network state.
    pub fn update_credits_and_debits<A: Accountable>(
        &mut self,
        txns: &LinkedHashMap<String, A>,
        reward: Reward,
    ) {
        let chs = self.clone().credit_hash(txns, reward);
        let dhs = self.clone().debit_hash(txns);
        self.credits = Some(chs);
        self.debits = Some(dhs);
    }

    /// Updates the state hash
    pub fn update_state_hash(&mut self, hash: &StateHash) {
        self.state_hash = Some(hash.clone());
    }

    /// Returns the credits from the ledger
    pub fn get_credits(&self) -> LinkedHashMap<String, u128> {
        Ledger::<Claim>::from_bytes(self.ledger.clone()).credits
    }

    /// Returns the debits from the ledger
    pub fn get_debits(&self) -> LinkedHashMap<String, u128> {
        Ledger::<Claim>::from_bytes(self.ledger.clone()).debits
    }

    /// Returns the claims from the ledger
    pub fn get_claims(&self) -> LinkedHashMap<String, Claim> {
        Ledger::<Claim>::from_bytes(self.ledger.clone()).claims
    }

    /// It returns the reward of the current state.
    pub fn get_reward(&self) -> Option<Reward> {
        self.reward.clone()
    }

    /// Gets the credits from a specific account
    pub fn get_account_credits(&self, address: &str) -> u128 {
        let credits = self.get_credits();
        if let Some(amount) = credits.get(address) {
            *amount
        } else {
            0u128
        }
    }

    /// Gets the debits from a specific account
    pub fn get_account_debits(&self, address: &str) -> u128 {
        let debits = self.get_debits();
        if let Some(amount) = debits.get(address) {
            *amount
        } else {
            0u128
        }
    }

    /// Replaces the current ledger with a new ledger
    pub fn update_ledger(&mut self, ledger: Ledger<Claim>) {
        self.ledger = ledger.as_bytes();
    }

    /// Calculates the lowest pointer sums given the claim map
    pub fn get_lowest_pointer(&self, block_seed: u128) -> Option<(String, u128)> {
        let claim_map = self.get_claims();
        let mut pointers = claim_map
            .iter()
            .map(|(_, claim)| (claim.clone().hash, claim.clone().get_pointer(block_seed)))
            .collect::<Vec<_>>();

        pointers.retain(|(_, v)| !v.is_none());

        let mut base_pointers = pointers
            .iter()
            .map(|(k, v)| (k.clone(), v.unwrap()))
            .collect::<Vec<_>>();

        if let Some(min) = base_pointers.clone().iter().min_by_key(|(_, v)| v) {
            base_pointers.retain(|(_, v)| *v == min.1);
            Some(base_pointers[0].clone())
        } else {
            None
        }
    }

    /// Slashes a claim of a miner that proposes an invalid block or spams the
    /// network
    pub fn slash_claims(&mut self, bad_validators: Vec<String>) {
        let mut ledger: Ledger<Claim> = Ledger::from_bytes(self.ledger.clone());
        bad_validators.iter().for_each(|k| {
            if let Some(claim) = ledger.claims.get_mut(&k.to_string()) {
                claim.eligible = false;
            }
        });
        self.ledger = ledger.as_bytes();
        self.dump_to_file()
    }

    /// Dumps a hex string representation of the `NetworkState` to file.
    pub fn dump_to_file(&self) {
        if fs::write(self.path.clone(), hex::encode(self.as_bytes())).is_err() {
            info!("Error dumping ledger to file");
        };
    }

    /// Returns a serialized representation of the credits map as a vector of
    /// bytes
    pub fn credits_as_bytes(credits: &LinkedHashMap<String, u128>) -> Vec<u8> {
        NetworkState::credits_to_string(credits).as_bytes().to_vec()
    }

    /// Returns a serialized representation of the credits map as a string
    pub fn credits_to_string(credits: &LinkedHashMap<String, u128>) -> String {
        serde_json::to_string(credits).unwrap()
    }

    /// Returns a credits map from a byte array
    pub fn credits_from_bytes(data: &[u8]) -> LinkedHashMap<String, u128> {
        serde_json::from_slice::<LinkedHashMap<String, u128>>(data).unwrap()
    }

    /// Returns a vector of bytes representing the debits map
    pub fn debits_as_bytes(debits: &LinkedHashMap<String, u128>) -> Vec<u8> {
        NetworkState::debits_to_string(debits).as_bytes().to_vec()
    }

    /// Returns a string representing the debits map
    pub fn debits_to_string(debits: &LinkedHashMap<String, u128>) -> String {
        serde_json::to_string(debits).unwrap()
    }

    /// Converts a byte array representing the debits map back into the debits
    /// map
    pub fn debits_from_bytes(data: &[u8]) -> LinkedHashMap<String, u128> {
        serde_json::from_slice::<LinkedHashMap<String, u128>>(data).unwrap()
    }

    /// Returns a vector of bytes representing the claim map
    pub fn claims_as_bytes<C: Ownable + Serialize>(claims: &LinkedHashMap<u128, C>) -> Vec<u8> {
        NetworkState::claims_to_string(claims).as_bytes().to_vec()
    }

    /// Returns a string representation of the claim map
    pub fn claims_to_string<C: Ownable + Serialize>(claims: &LinkedHashMap<u128, C>) -> String {
        serde_json::to_string(claims).unwrap()
    }

    /// Returns a claim map from an array of bytes
    pub fn claims_from_bytes<C: Ownable + Deserialize<'de>>(
        data: &'de [u8],
    ) -> LinkedHashMap<u128, C> {
        serde_json::from_slice::<LinkedHashMap<u128, C>>(data).unwrap()
    }

    /// Returns a block (representing the last block) from a byte array
    pub fn last_block_from_bytes<D: Deserialize<'de>>(data: &'de [u8]) -> D {
        serde_json::from_slice::<D>(data).unwrap()
    }

    /// Serializes the network state as a vector of bytes
    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_string().as_bytes().to_vec()
    }

    /// Converts a vector of bytes into a Network State or returns an error if
    /// it's unable to
    pub fn from_bytes(data: Vec<u8>) -> Result<NetworkState, serde_json::error::Error> {
        serde_json::from_slice::<NetworkState>(&data)
    }

    /// Serializes the network state into a string
    // TODO: Consider moving this to Display trait
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    /// Deserializes the network state from a string
    pub fn from_string(string: String) -> NetworkState {
        serde_json::from_str::<NetworkState>(&string).unwrap()
    }

    /// creates a Ledger from the network state
    pub fn db_to_ledger(&self) -> Ledger<Claim> {
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

impl Clone for NetworkState {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            ledger: self.ledger.clone(),
            credits: self.credits.clone(),
            debits: self.debits.clone(),
            reward: self.reward.clone(),
            state_hash: self.state_hash.clone(),
        }
    }
}
