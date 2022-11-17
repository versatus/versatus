// This file contains code for creating blocks to be proposed, including the
// genesis block and blocks being mined.
#![allow(unused_imports)]
#![allow(dead_code)]
use std::{fmt, f32::consts::E};

use accountable::accountable::Accountable;
use claim::claim::Claim;
use log::info;
use primitives::types::RawSignature;
use rand::Rng;
use reward::reward::{Category, RewardState, GENESIS_REWARD};
use ritelinked::LinkedHashMap;
use serde::{Deserialize, Serialize};
use sha256::digest_bytes;
use state::state::NetworkState;
use txn::txn::Txn;
use verifiable::verifiable::Verifiable;

use crate::header::BlockHeader;


use thiserror::Error;

pub const NANO: u128 = 1;
pub const MICRO: u128 = NANO * 1000;
pub const MILLI: u128 = MICRO * 1000;
pub const SECOND: u128 = MILLI * 1000;

const VALIDATOR_THRESHOLD: f64 = 0.60;


#[derive(Debug, Error)]
pub enum BlockError {
    #[error("blockchain proposed is shorter than my local chain")]
    NotTallestChain(),
   
    #[error("block out of sequence")]
    BlockOutOfSequence(),

    #[error("invalid claim")]
    InvalidClaim(),

    #[error("invalid last hash")]
    InvalidLastHash(),
    
    #[error("invalid state hash")]
    InvalidStateHash(),

    #[error("invalid block height")]
    InvalidBlockHeight(),

    #[error("invalid block seed")]
    InvalidBlockSeed(),

    #[error("invalid block reward")]
    InvalidBlockReward(),

    #[error("invalid txns in block")]
    InvalidTxns(),

    #[error("invalid claim pointers")]
    InvalidClaimPointers(),

    #[error("invalid next block reward")]
    InvalidNextBlockReward(),

    #[error("invalid block signature")]
    InvalidBlockSignature(),

    #[error("general block error")]
    General(),

    #[error("invalid header seed generated")]
    InvalidHeaderSeed(),

    #[error("invalid block header")]
    InvalidBlockHeader(),
}


#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(C)]
pub struct Block {
    pub header: BlockHeader,
    pub neighbors: Option<Vec<BlockHeader>>,
    pub height: u128,
    // TODO: replace with Tx Trie Root
    pub txns: LinkedHashMap<String, Txn>,
    // TODO: Replace with Claim Trie Root
    pub claims: LinkedHashMap<String, Claim>,
    pub hash: Vec<u8>,
    pub received_at: Option<u128>,
    pub received_from: Option<String>,
    // TODO: Replace with map of all abandoned claims in the even more than 1 miner is faulty when
    // they are entitled to mine
    pub abandoned_claim: Option<Claim>,
    // Quorum signature needed for finalizing the block and locking the chain
    pub threshold_signature: Option<RawSignature>,
}

impl Block {
    // Returns a result with either a tuple containing the genesis block and the
    // updated account state (if successful) or an error (if unsuccessful)
    pub fn genesis(reward_state: &RewardState, claim: Claim, secret_key: String) -> Option<Block> {
        // Create the genesis header
        let mut header = (BlockHeader::genesis(0, reward_state, claim.clone(), secret_key)).unwrap();

       // let header = match BlockHeader::genesis(0, reward_state, claim.clone(), secret_key){
       //     Ok(header) => header,
       //     Err(e) => Err(e),
       // };
       // let header = BlockHeader::genesis(0, reward_state, claim.clone(), secret_key);
        // Create the genesis state hash
        // TODO: Replace with state trie root
        let mut genesis_state_hash = "Genesis_State_Hash".as_bytes().to_vec();
        header.last_hash.append(&mut genesis_state_hash);
        let state_hash = header.last_hash;

        // Replace with claim trie
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
            threshold_signature: None,
        };

        // Update the State Trie & Tx Trie with the miner and new block, this will also
        // set the values to the network state. Unwrap the result and assign it
        // to the variable updated_account_state to be returned by this method.

        Some(genesis)
    }

    /// The mine method is used to generate a new block (and an updated account
    /// state with the reward set to the miner wallet's balance), this will
    /// also update the network state with a new confirmed state.
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
        // TODO: Replace with Tx Trie Root
        let txn_hash = {
            let mut txn_vec = vec![];
            txns.iter().for_each(|(_, v)| {
                txn_vec.extend(v.as_bytes());
            });
            digest_bytes(&txn_vec)
        };

        // TODO: Remove there should be no neighbors
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

        // TODO: Fix after replacing neighbors and tx hash/claim hash with respective
        // Trie Roots
        let header = (BlockHeader::new(
            last_block.clone(),
            reward_state,
            claim,
            txn_hash,
            claim_map_hash,
            neighbors_hash,
            signature,
        )).unwrap();

        // TODO: Discuss whether local clock works well enough for this purpose of
        // guaranteeing at least 1 second between blocks or whether some other
        // mechanism may serve the purpose better, or whether simply sequencing proposed
        // blocks and allowing validator network to determine how much time
        // between blocks has passed.
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
            threshold_signature: None,
        };

        // TODO: Replace with state trie
        let mut hashable_state = network_state.clone();

        let hash = hashable_state.hash(&block.txns.clone(), block.header.block_reward.clone()).into_bytes();
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

// TODO: Rewrite Verifiable to comport with Masternode Quorum Validation
// Protocol
impl Verifiable for Block {
    type Dependencies = (NetworkState, RewardState);
    type Error = BlockError;
    type Item = Block;

    fn verifiable(&self) -> bool {
        true
    }

    #[allow(unused_variables)]
    fn valid(
        &self,
        item: &Self::Item,
        dependencies: &Self::Dependencies,
    ) -> Result<bool, Self::Error> {
        if self.header.block_height > item.header.block_height + 1 {
            return Err(BlockError::BlockOutOfSequence());
        }

        if self.header.block_height < item.header.block_height
            || self.header.block_height == item.header.block_height
        {
            return Err(BlockError::NotTallestChain());
        }

        if self.header.block_seed != item.header.next_block_seed {
            return Err(BlockError::InvalidBlockSeed());
        }

        if self.header.block_reward.category != item.header.next_block_reward.category {
            return Err(BlockError::InvalidBlockReward());
        }

        if self.header.block_reward.get_amount() != item.header.next_block_reward.get_amount() {
            return Err(BlockError::InvalidBlockReward());
        }

        if let Some((hash, pointers)) = dependencies
            .0
            .get_lowest_pointer(self.header.block_seed as u128)
        {
            if hash == self.header.claim.hash {
                if let Some(claim_pointer) = self
                    .header
                    .claim
                    .get_pointer(self.header.block_seed as u128)
                {
                    if pointers != claim_pointer {
                        return Err( BlockError::InvalidClaimPointers());
                    }
                } else {
                    return Err(BlockError::InvalidClaimPointers());
                }
            } else {
                return Err(BlockError::InvalidClaimPointers());
            }
        }

        if !dependencies
            .1
            .valid_reward(self.header.block_reward.category)
        {
            return Err(BlockError::InvalidBlockReward());
        }

        if !dependencies
            .1
            .valid_reward(self.header.next_block_reward.category)
        {
            return Err( BlockError::InvalidNextBlockReward());
        }

        if self.header.last_hash != item.hash {
            return Err(BlockError::InvalidLastHash());
        }

        if let Err(_) = self.header.claim.valid(&None, &(None, None)) {
            return Err(BlockError::InvalidClaim());
        }

        Ok(true)
    }

    fn valid_genesis(&self, dependencies: &Self::Dependencies) -> Result<bool, Self::Error> {
        let genesis_last_hash = digest_bytes("Genesis_Last_Hash".as_bytes()).into_bytes();
        let genesis_msg = "Genesis_State_Hash".as_bytes().to_vec();
        genesis_last_hash.append(&mut genesis_msg);
        let genesis_state_hash = genesis_last_hash;
        
        if self.header.block_height != 0 {
            return Err(BlockError::InvalidBlockHeight());
        }

        if !dependencies
            .1
            .valid_reward(self.header.block_reward.category)
        {
            return Err(BlockError::InvalidBlockReward());
        }

        if !dependencies
            .1
            .valid_reward(self.header.next_block_reward.category)
        {
            return Err(BlockError::InvalidNextBlockReward());
        }

        if String::from_utf8(self.header.last_hash) != String::from_utf8(genesis_last_hash) {
            return Err(BlockError::InvalidLastHash());
        }

        if String::from_utf8(self.hash) != String::from_utf8(genesis_state_hash) {
            return Err(BlockError::InvalidStateHash());
        }

        if let Err(_) = self.header.claim.valid(&None, &(None, None)) {
            return Err(BlockError::InvalidClaim());
        }

        if let Err(_) = self.header.verify() {
            return Err(BlockError::InvalidBlockSignature());
        }

        let mut valid_data = true;
        self.txns.iter().for_each(|(_, txn)| {
            let n_valid = txn.validators.iter().filter(|(_, &valid)| valid).count();
            if (n_valid as f64 / txn.validators.len() as f64) < VALIDATOR_THRESHOLD {
                valid_data = false;
            }
        });

        if !valid_data {
            return Err(BlockError::InvalidTxns());
        }

        Ok(true)
    }
}
