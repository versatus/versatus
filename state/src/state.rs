use accountable::accountable::Accountable;
use claim::claim::Claim;
use ledger::ledger::Ledger;
use log::info;
use noncing::nonceable::Nonceable;
use ownable::ownable::Ownable;
use reward::reward::{Reward, RewardState};
use ritelinked::LinkedHashMap;
use serde::{Deserialize, Serialize};
use sha256::digest_bytes;
use std::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StateUpdateHead(pub u16);

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
    // ledger.as_bytes()
    pub ledger: Vec<u8>,
    // hash of the state of credits in the network
    pub credits: Option<String>,
    // hash of the state of debits in the network
    pub debits: Option<String>,
    //reward state of the network
    pub reward_state: Option<RewardState>,
    // the last state hash -> sha256 hash of credits, debits & reward state.
    pub state_hash: Option<String>,
}

impl<'de> NetworkState {
    pub fn restore(path: &str) -> NetworkState {
        let hex_string = {
            if let Ok(string) = fs::read_to_string(path) {
                string
            } else {
                String::new()
            }
        };
        let bytes = hex::decode(hex_string.clone());
        if let Ok(state_bytes) = bytes {
            if let Ok(network_state) = NetworkState::from_bytes(state_bytes) {
                network_state.dump_to_file();
                return network_state;
            }
        }

        let network_state = NetworkState {
            path: path.to_string(),
            ledger: vec![],
            credits: None,
            debits: None,
            reward_state: None,
            state_hash: None,
        };

        network_state.dump_to_file();

        network_state
    }

    pub fn set_ledger(&mut self, ledger_bytes: Vec<u8>) {
        self.ledger = ledger_bytes;
        self.dump_to_file();
    }

    pub fn set_reward_state(&mut self, reward_state: RewardState) {
        self.reward_state = Some(reward_state);
        self.dump_to_file();
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

    pub fn credit_hash<A: Accountable, R: Accountable>(
        self,
        txns: &LinkedHashMap<String, A>,
        reward: R,
    ) -> String {
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
            return digest_bytes(format!("{},{:?}", chs, credits).as_bytes());
        } else {
            return digest_bytes(format!("{:?},{:?}", self.credits, credits).as_bytes());
        }
    }

    pub fn debit_hash<A: Accountable>(self, txns: &LinkedHashMap<String, A>) -> String {
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
            return digest_bytes(format!("{},{:?}", dhs, debits).as_bytes());
        } else {
            return digest_bytes(format!("{:?},{:?}", self.debits, debits).as_bytes());
        }
    }

    pub fn hash<A: Accountable, R: Accountable>(
        &mut self,
        txns: &LinkedHashMap<String, A>,
        reward: R,
    ) -> String {
        let credit_hash = self.clone().credit_hash(&txns, reward);
        let debit_hash = self.clone().debit_hash(&txns);
        let reward_state_hash = digest_bytes(format!("{:?}", self.reward_state).as_bytes());
        let payload = format!(
            "{:?},{:?},{:?},{:?}",
            self.state_hash, credit_hash, debit_hash, reward_state_hash
        );
        let new_state_hash = digest_bytes(payload.as_bytes());
        new_state_hash
    }

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

        self.update_reward_state(reward.clone());
        self.update_state_hash(hash);
        self.update_credits_and_debits(&txns, reward.clone());

        let ledger_hex = hex::encode(ledger.clone().as_bytes());
        if let Err(_) = fs::write(self.path.clone(), ledger_hex) {
            info!("Error writing ledger hex to file");
        };

        self.ledger = ledger.as_bytes();
    }

    // TODO: refactor to handle NetworkState nonce_up() a different way, since
    // closure requires explicit types and explicit type specification would
    // lead to cyclical dependencies.
    pub fn nonce_up(&mut self) {
        let mut new_claim_map: LinkedHashMap<String, Claim> = LinkedHashMap::new();
        let claims: LinkedHashMap<String, Claim> = self.get_claims().clone();
        claims.iter().for_each(|(pk, claim)| {
            let mut new_claim = claim.clone();
            new_claim.nonce_up();
            new_claim_map.insert(pk.clone(), new_claim.clone());
        });

        let mut ledger = Ledger::from_bytes(self.ledger.clone());
        ledger.claims = new_claim_map;
        self.ledger = ledger.as_bytes();
    }

    pub fn abandoned_claim(&mut self, hash: String) {
        let mut ledger: Ledger<Claim> = Ledger::from_bytes(self.ledger.clone());
        ledger.claims.retain(|_, v| v.hash != hash);
        self.ledger = ledger.as_bytes();
        self.dump_to_file();
    }

    pub fn restore_ledger(&self) -> Ledger<Claim> {
        let network_state_hex = fs::read_to_string(self.path.clone()).unwrap();
        let bytes = hex::decode(network_state_hex);
        if let Ok(state_bytes) = bytes {
            if let Ok(network_state) = NetworkState::from_bytes(state_bytes) {
                return Ledger::from_bytes(network_state.ledger.clone());
            } else {
                return Ledger::new();
            }
        } else {
            return Ledger::new();
        }
    }

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

    pub fn update_reward_state(&mut self, reward: Reward) {
        if let Some(category) = reward.get_category() {
            if let Some(mut reward_state) = self.reward_state.clone() {
                reward_state.update(category);
                self.reward_state = Some(reward_state);
            }
        }
    }

    pub fn update_state_hash(&mut self, hash: &String) {
        self.state_hash = Some(hash.clone());
    }

    pub fn get_credits(&self) -> LinkedHashMap<String, u128> {
        Ledger::<Claim>::from_bytes(self.ledger.clone())
            .credits
            .clone()
    }

    pub fn get_debits(&self) -> LinkedHashMap<String, u128> {
        Ledger::<Claim>::from_bytes(self.ledger.clone())
            .debits
            .clone()
    }

    pub fn get_claims(&self) -> LinkedHashMap<String, Claim> {
        Ledger::<Claim>::from_bytes(self.ledger.clone())
            .claims
            .clone()
    }

    pub fn get_reward_state(&self) -> Option<RewardState> {
        self.reward_state.clone()
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
    pub fn update_ledger(&mut self, ledger: Ledger<Claim>) {
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
        let mut ledger: Ledger<Claim> = Ledger::from_bytes(self.ledger.clone());
        bad_validators.iter().for_each(|k| {
            if let Some(claim) = ledger.claims.get_mut(&k.to_string()) {
                claim.eligible = false;
            }
        });
        self.ledger = ledger.as_bytes();
        self.dump_to_file()
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

    pub fn claims_as_bytes<C: Ownable + Serialize>(claims: &LinkedHashMap<u128, C>) -> Vec<u8> {
        NetworkState::claims_to_string(claims).as_bytes().to_vec()
    }

    pub fn claims_to_string<C: Ownable + Serialize>(claims: &LinkedHashMap<u128, C>) -> String {
        serde_json::to_string(claims).unwrap()
    }

    pub fn claims_from_bytes<C: Ownable + Deserialize<'de>>(
        data: &'de [u8],
    ) -> LinkedHashMap<u128, C> {
        serde_json::from_slice::<LinkedHashMap<u128, C>>(data).unwrap()
    }

    pub fn last_block_from_bytes<D: Deserialize<'de>>(data: &'de [u8]) -> D {
        serde_json::from_slice::<D>(data).unwrap()
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_string().as_bytes().to_vec()
    }

    pub fn from_bytes(data: Vec<u8>) -> Result<NetworkState, serde_json::error::Error> {
        serde_json::from_slice::<NetworkState>(&data.clone())
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn from_string(string: String) -> NetworkState {
        serde_json::from_str::<NetworkState>(&string).unwrap()
    }

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

impl StateUpdateHead {
    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_string().as_bytes().to_vec()
    }

    pub fn from_bytes(data: &[u8]) -> Option<StateUpdateHead> {
        if let Ok(state_update_head) = serde_json::from_slice::<StateUpdateHead>(data) {
            Some(state_update_head)
        } else {
            None
        }
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn from_string(string: &String) -> StateUpdateHead {
        serde_json::from_str::<StateUpdateHead>(&string).unwrap()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_new_network_state() {
        // TODO: implement
    }
}
