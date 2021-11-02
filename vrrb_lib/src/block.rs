use crate::blockchain::{InvalidBlockError, InvalidBlockErrorReason};
use crate::header::BlockHeader;
use crate::state::NetworkState;
use crate::verifiable::Verifiable;
use crate::{claim::Claim, reward::RewardState, txn::Txn};
use log::info;
use ritelinked::LinkedHashMap;
use serde::{Deserialize, Serialize};
use sha256::digest_bytes;
use std::fmt;

pub const NANO: u128 = 1;
pub const MICRO: u128 = NANO * 1000;
pub const MILLI: u128 = MICRO * 1000;
pub const SECOND: u128 = MILLI * 1000;

const VALIDATOR_THRESHOLD: f64 = 0.60;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(C)]
pub struct Block {
    pub header: BlockHeader,
    pub neighbors: Option<Vec<BlockHeader>>,
    pub height: u128,
    pub txns: LinkedHashMap<String, Txn>,
    pub claims: LinkedHashMap<String, Claim>,
    pub hash: String,
    pub received_at: Option<u128>,
    pub received_from: Option<String>,
    pub abandoned_claim: Option<Claim>,
}

impl Block {
    // Returns a result with either a tuple containing the genesis block and the
    // updated account state (if successful) or an error (if unsuccessful)
    pub fn genesis(reward_state: &RewardState, claim: Claim, secret_key: String) -> Option<Block> {
        let header = BlockHeader::genesis(0, reward_state, claim.clone(), secret_key);
        let state_hash = digest_bytes(
            format!(
                "{},{}",
                header.last_hash,
                digest_bytes("Genesis_State_Hash".as_bytes())
            )
            .as_bytes(),
        );

        let mut claims = LinkedHashMap::new();
        claims.insert(claim.clone().pubkey.clone(), claim);

        let genesis = Block {
            header,
            neighbors: None,
            height: 0,
            txns: LinkedHashMap::new(),
            claims,
            hash: state_hash,
            received_at: None,
            received_from: None,
            abandoned_claim: None,
        };

        // Update the account state with the miner and new block, this will also set the values to the
        // network state. Unwrap the result and assign it to the variable updated_account_state to
        // be returned by this method.

        Some(genesis)
    }

    /// The mine method is used to generate a new block (and an updated account state with the reward set
    /// to the miner wallet's balance), this will also update the network state with a new confirmed state.
    pub fn mine(
        claim: Claim,      // The claim entitling the miner to mine the block.
        last_block: Block, // The last block, which contains the current block reward.
        txns: LinkedHashMap<String, Txn>,
        claims: LinkedHashMap<String, Claim>,
        claim_map_hash: Option<String>,
        reward_state: &RewardState,
        network_state: &NetworkState,
        neighbors: Option<Vec<BlockHeader>>,
        abandoned_claim: Option<Claim>,
        signature: String,
    ) -> Option<Block> {
        let txn_hash = {
            let mut txn_vec = vec![];
            txns.iter().for_each(|(_, v)| {
                txn_vec.extend(v.as_bytes());
            });
            digest_bytes(&txn_vec)
        };

        let neighbors_hash = {
            let mut neighbors_vec = vec![];
            if let Some(neighbors) = &neighbors {
                neighbors.iter().for_each(|v| {
                    neighbors_vec.extend(v.as_bytes());
                });
                Some(digest_bytes(&neighbors_vec))
            } else {
                None
            }
        };

        let header = BlockHeader::new(
            last_block.clone(),
            reward_state,
            claim,
            txn_hash,
            claim_map_hash,
            neighbors_hash,
            signature,
        );

        if let Some(time) = header.timestamp.checked_sub(last_block.header.timestamp) {
            if (time / SECOND) < 1 {
                return None;
            }
        } else {
            return None;
        }

        let height = last_block.height.clone() + 1;

        let mut block = Block {
            header: header.clone(),
            neighbors,
            height,
            txns,
            claims,
            hash: header.last_hash.clone(),
            received_at: None,
            received_from: None,
            abandoned_claim,
        };

        let mut hashable_state = network_state.clone();

        let hash = hashable_state.hash(block.clone());
        block.hash = hash;
        Some(block)
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_string().as_bytes().to_vec()
    }

    pub fn from_bytes(data: &[u8]) -> Block {
        let mut buffer: Vec<u8> = vec![];

        data.iter().for_each(|x| buffer.push(*x));

        let to_string = String::from_utf8(buffer).unwrap();

        serde_json::from_str::<Block>(&to_string).unwrap()
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Block(\n \
            header: {:?},\n",
            self.header
        )
    }
}

impl Verifiable for Block {
    fn verifiable(&self) -> bool {
        true
    }

    fn valid_genesis(&self, _network_state: &NetworkState, _reward_state: &RewardState) -> bool {
        true
    }

    fn valid_block(
        &self,
        last_block: &Block,
        network_state: &NetworkState,
        reward_state: &RewardState,
    ) -> Result<(), InvalidBlockError> {
        if !self.valid_last_hash(last_block) {
            let e = Err(InvalidBlockError {
                details: InvalidBlockErrorReason::InvalidLastHash,
            });
            info!("Invalid block: {:?}", e);
            info!("Block that's invalid: {:?}", self);
            info!("Last Valid Block: {:?}", &last_block);
            return e;
        }

        if !self.valid_block_nonce(last_block) {
            let e = Err(InvalidBlockError {
                details: InvalidBlockErrorReason::InvalidBlockNonce,
            });
            info!("Invalid block: {:?}", e);
            info!("Block that's invalid: {:?}", self);
            info!("Last Valid Block: {:?}", &last_block);
            return e;
        }

        if !self.valid_state_hash(network_state) {
            let e = Err(InvalidBlockError {
                details: InvalidBlockErrorReason::InvalidStateHash,
            });
            info!("Invalid block: {:?}", e);
            info!("Block that's invalid: {:?}", self);
            info!("Last Valid Block: {:?}", &last_block);
            return e;
        }

        if !self.valid_block_reward(reward_state) {
            let e = Err(InvalidBlockError {
                details: InvalidBlockErrorReason::InvalidBlockReward,
            });
            info!("Invalid block: {:?}", e);
            info!("Block that's invalid: {:?}", self);
            info!("Last Valid Block: {:?}", &last_block);
            return e;
        }

        if !self.valid_next_block_reward(reward_state) {
            let e = Err(InvalidBlockError {
                details: InvalidBlockErrorReason::InvalidBlockReward,
            });
            info!("Invalid block: {:?}", e);
            info!("Block that's invalid: {:?}", self);
            info!("Last Valid Block: {:?}", &last_block);
            return e;
        }

        if !self.valid_txns() {
            let e = Err(InvalidBlockError {
                details: InvalidBlockErrorReason::InvalidTxns,
            });
            info!("Invalid block: {:?}", e);
            info!("Block that's invalid: {:?}", self);
            info!("Last Valid Block: {:?}", &last_block);
            return e;
        }

        if !self.valid_claim_pointer(network_state) {
            let e = Err(InvalidBlockError {
                details: InvalidBlockErrorReason::InvalidClaimPointers,
            });
            info!("Invalid block: {:?}", e);
            info!("Block that's invalid: {:?}", self);
            info!("Last Valid Block: {:?}", &last_block);
            return e;
        }

        if !self.valid_block_claim(network_state) {
            let e = Err(InvalidBlockError {
                details: InvalidBlockErrorReason::InvalidClaim,
            });
            info!("Invalid block: {:?}", e);
            info!("Block that's invalid: {:?}", self);
            info!("Last Valid Block: {:?}", &last_block);
            return e;
        }

        Ok(())
    }

    fn valid_last_hash(&self, last_block: &Block) -> bool {
        self.header.last_hash == last_block.hash
    }

    fn valid_state_hash(&self, network_state: &NetworkState) -> bool {
        let mut hashable_state = network_state.clone();
        let hash = hashable_state.hash(self.clone());
        self.hash == hash
    }

    fn valid_block_reward(&self, reward_state: &RewardState) -> bool {
        if let Some(true) = reward_state.valid_reward(self.header.block_reward.category) {
            return true;
        }

        false
    }

    fn valid_next_block_reward(&self, reward_state: &RewardState) -> bool {
        if let Some(true) = reward_state.valid_reward(self.header.next_block_reward.category) {
            return true;
        }

        false
    }

    fn valid_txns(&self) -> bool {
        let mut valid_data: bool = true;

        self.txns.iter().for_each(|(_, txn)| {
            let n_valid = txn.validators.iter().filter(|(_, &valid)| valid).count();
            if (n_valid as f64 / txn.validators.len() as f64) < VALIDATOR_THRESHOLD {
                valid_data = false
            }
        });

        valid_data
    }

    fn valid_block_nonce(&self, last_block: &Block) -> bool {
        self.header.block_nonce == last_block.header.next_block_nonce
    }

    fn valid_claim_pointer(&self, network_state: &NetworkState) -> bool {
        if let Some((hash, pointers)) =
            network_state.get_lowest_pointer(self.header.block_nonce as u128)
        {
            if hash == self.header.claim.hash {
                if let Some(claim_pointer) = self
                    .header
                    .claim
                    .get_pointer(self.header.block_nonce as u128)
                {
                    if pointers == claim_pointer {
                        return true;
                    } else {
                        return false;
                    }
                } else {
                    return false;
                }
            } else {
                return false;
            }
        } else {
            return false;
        }
    }

    fn valid_block_signature(&self) -> bool {
        if let Ok(true) = self.header.verify() {
            return true;
        } else {
            return false;
        }
    }

    fn valid_block_claim(&self, network_state: &NetworkState) -> bool {
        let claims = network_state.get_claims();
        if let None = claims.get(&self.header.claim.pubkey) {
            return false;
        }
        let recreated_claim = Claim::new(
            self.header.claim.pubkey.clone(),
            self.header.claim.address.clone(),
            self.header.claim.nonce,
        );

        if recreated_claim.hash != self.header.claim.hash {
            info!("Claim hash is incorrect, doesn't match recreated claim hash");
            return false;
        }

        let network_state_claim = claims.get(&self.header.claim.pubkey).unwrap();

        if network_state_claim.pubkey != self.header.claim.pubkey {
            info!("Claim pubkey doesn't match records");
            return false;
        }
        if network_state_claim.address != self.header.claim.address {
            info!("Claim address doesn't match records");
            return false;
        }

        if network_state_claim.hash != self.header.claim.hash {
            info!("Claim hash doesn't match records");
            return false;
        }

        if network_state_claim.nonce != self.header.claim.nonce {
            info!("Claim nonce doesn't match records");
            return false;
        }

        return true;
    }
}
