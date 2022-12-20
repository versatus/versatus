// FEATURE TAG(S): Block Structure, Rewards
use std::{
    time::{SystemTime, UNIX_EPOCH},
    u32::MAX as u32MAX,
    u64::MAX as u64MAX,
};

use rand::Rng;
use reward::reward::Reward;
use serde::{Deserialize, Serialize};
use sha256::digest;
use vrrb_core::{claim::Claim, keypair::KeyPair};

use crate::{block::Block, NextEpochAdjustment};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    // TODO: Rename block_nonce and next_block_nonce to block_seed and next_block_seed respectively
    // TODO: Replace tx hash with tx trie root
    // TODO: Replace claim hash with claim trie root
    // TODO: Add certificate field for validation certification.
    pub last_hash: String,
    pub block_nonce: u64,
    pub next_block_nonce: u64,
    pub block_height: u128,
    pub timestamp: u128,
    pub txn_hash: String,
    pub claim: Claim,
    pub claim_map_hash: Option<String>,
    pub block_reward: Reward,
    pub next_block_reward: Reward,
    pub neighbor_hash: Option<String>,
    pub signature: String,
}

impl BlockHeader {
    pub fn genesis(
        nonce: u64,
        claim: Claim,
        secret_key: Vec<u8>,
        miner: Option<String>,
    ) -> BlockHeader {
        //TODO: Replace rand::thread_rng() with VPRNG
        //TODO: Determine data fields to be used as message in VPRNG, must be
        // known/revealed within block but cannot be predictable or gameable.
        // Leading candidates are some combination of last_hash and last_block_seed
        let mut rng = rand::thread_rng();
        let last_hash = digest("Genesis_Last_Hash".as_bytes());
        let block_nonce = nonce;
        // Range should remain the same.
        let next_block_nonce: u64 = rng.gen_range(u32MAX as u64, u64MAX);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let txn_hash = digest("Genesis_Txn_Hash".as_bytes());
        let block_reward = Reward::genesis(Some(claim.address.clone()));
        //TODO: Replace reward state
        let next_block_reward = Reward::genesis(miner);
        let claim_map_hash: Option<String> = None;
        let neighbor_hash: Option<String> = None;
        let payload = format!(
            "{},{},{},{},{},{},{:?},{:?},{:?},{:?},{:?}",
            last_hash,
            block_nonce,
            next_block_nonce,
            0,
            timestamp,
            txn_hash,
            claim,
            claim_map_hash,
            block_reward,
            next_block_reward,
            neighbor_hash,
        );

        let signature = KeyPair::edsca_sign(payload.as_bytes(), secret_key).unwrap();

        BlockHeader {
            last_hash,
            block_nonce,
            next_block_nonce,
            block_height: 0,
            timestamp,
            txn_hash,
            claim,
            claim_map_hash: None,
            block_reward,
            next_block_reward,
            neighbor_hash: None,
            signature,
        }
    }

    pub fn new(
        last_block: Block,
        reward: &mut Reward,
        claim: Claim,
        txn_hash: String,
        claim_map_hash: Option<String>,
        neighbor_hash: Option<String>,
        secret_key: Vec<u8>,
        epoch_change: bool,
        adjustment_next_epoch: NextEpochAdjustment,
    ) -> BlockHeader {
        //TODO: Replace rand::thread_rng() with VPRNG
        //TODO: Determine data fields to be used as message in VPRNG, must be
        // known/revealed within block but cannot be predictable or gameable.
        // Leading candidates are some combination of last_hash and last_block_seed
        let mut rng = rand::thread_rng();
        let last_hash = last_block.hash;
        let block_nonce = last_block.header.next_block_nonce;
        let next_block_nonce: u64 = rng.gen_range(0, u64MAX);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let mut block_reward = last_block.header.next_block_reward;
        block_reward.miner = Some(claim.clone().address);

        let mut next_block_reward = reward.clone();
        if epoch_change {
            reward.new_epoch(adjustment_next_epoch);
            next_block_reward = reward.clone();
        }
        let block_height = last_block.header.block_height + 1;
        let payload = format!(
            "{},{},{},{},{},{},{:?},{:?},{:?},{:?},{:?}",
            last_hash,
            block_nonce,
            next_block_nonce,
            block_height,
            timestamp,
            txn_hash,
            claim,
            claim_map_hash,
            block_reward,
            next_block_reward,
            neighbor_hash,
        );
        let signature = KeyPair::edsca_sign(payload.as_bytes(), secret_key).unwrap();
        BlockHeader {
            last_hash,
            block_nonce,
            next_block_nonce,
            block_height: last_block.header.block_height + 1,
            timestamp,
            txn_hash,
            claim,
            claim_map_hash,
            block_reward,
            next_block_reward,
            neighbor_hash: None,
            signature,
        }
    }

    pub fn get_payload(&self) -> String {
        format!(
            "{},{},{},{},{},{},{:?},{:?},{:?},{:?},{:?}",
            self.last_hash,
            self.block_nonce,
            self.next_block_nonce,
            self.block_height,
            self.timestamp,
            self.txn_hash,
            self.claim,
            self.claim_map_hash,
            self.block_reward,
            self.next_block_reward,
            self.neighbor_hash,
        )
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_string().as_bytes().to_vec()
    }

    pub fn from_bytes(data: &[u8]) -> BlockHeader {
        serde_json::from_slice(data).unwrap()
    }

    // TODO: Consider renaming to `serialize_to_str`
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    //TODO: consider renaming to sth like `deserialize_from_str`
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(data: &str) -> BlockHeader {
        serde_json::from_str(data).unwrap()
    }
}
