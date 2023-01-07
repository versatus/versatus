// FEATURE TAG(S): Block Structure, Rewards
#![allow(unused_imports)]
use std::{
    time::{SystemTime, UNIX_EPOCH},
    u32::MAX as u32MAX,
    u64::MAX as u64MAX,
};

use primitives::SecretKey as SecretKeyBytes;
use rand::Rng;
use reward::reward::Reward;
use secp256k1::{
    hashes::{sha256 as s256, Hash},
    Message,
};
use serde::{Deserialize, Serialize};
use sha256::digest;
use utils::{create_payload, hash_data};
use vrrb_core::{claim::Claim, keypair::KeyPair};

use crate::{block::Block, ConvergenceBlock, NextEpochAdjustment};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    // TODO: Replace tx hash with tx trie root???
    // TODO: Replace claim hash with claim trie root???
    pub ref_hashes: Vec<String>,
    pub round: u128,
    pub block_seed: u64,
    pub next_block_seed: u64,
    pub block_height: u128,
    pub timestamp: u128,
    pub txn_hash: String,
    pub miner_claim: Claim,
    pub claim_list_hash: String,
    pub block_reward: Reward,
    pub next_block_reward: Reward,
    pub miner_signature: String,
}

impl BlockHeader {
    pub fn genesis(
        seed: u64,
        round: u128,
        miner_claim: Claim,
        secret_key: SecretKeyBytes,
        claim_list_hash: String,
    ) -> BlockHeader {
        //TODO: Replace rand::thread_rng() with VPRNG
        //TODO: Determine data fields to be used as message in VPRNG, must be
        // known/revealed within block but cannot be predictable or gameable.
        // Leading candidates are some combination of last_hash and last_block_seed
        let mut rng = rand::thread_rng();
        let ref_hashes = vec![hash_data!("Genesis_Last_Hash")];

        // Range should remain the same.
        let next_block_seed: u64 = rng.gen_range(u32MAX as u64, u64MAX);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let txn_hash = hash_data!("Genesis_Txn_Hash");
        let block_reward = Reward::genesis(Some(miner_claim.address.clone()));

        let next_block_reward = Reward::default();

        let payload = create_payload!(
            ref_hashes,
            seed,
            next_block_seed,
            0,
            timestamp,
            txn_hash,
            miner_claim,
            claim_list_hash,
            block_reward,
            next_block_reward
        );

        let miner_signature = secret_key.sign_ecdsa(payload).to_string();

        BlockHeader {
            ref_hashes,
            round,
            block_seed: 0,
            next_block_seed,
            block_height: 0,
            timestamp,
            txn_hash,
            miner_claim,
            claim_list_hash,
            block_reward,
            next_block_reward,
            miner_signature,
        }
    }

    pub fn new(
        last_block: ConvergenceBlock,
        ref_hashes: Vec<String>,
        round: u128,
        reward: &mut Reward,
        miner_claim: Claim,
        secret_key: SecretKeyBytes,
        txn_hash: String,
        claim_list_hash: String,
        epoch_change: bool,
        adjustment_next_epoch: NextEpochAdjustment,
    ) -> BlockHeader {
        //TODO: Replace rand::thread_rng() with VPRNG
        //TODO: Determine data fields to be used as message in VPRNG, must be
        // known/revealed within block but cannot be predictable or gameable.
        // Leading candidates are some combination of last_hash and last_block_seed
        let mut rng = rand::thread_rng();

        let block_seed = last_block.header.next_block_seed;

        let next_block_seed: u64 = rng.gen_range(0, u64MAX);

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let mut block_reward = last_block.header.next_block_reward;

        block_reward.miner = Some(miner_claim.clone().address);

        let mut next_block_reward = block_reward.clone();

        if epoch_change {
            reward.new_epoch(adjustment_next_epoch);
            next_block_reward = reward.clone();
        }

        let block_height = last_block.header.block_height + 1;

        next_block_reward.current_block = reward.current_block + 1;

        let payload = create_payload!(
            ref_hashes,
            block_seed,
            next_block_seed,
            block_height,
            timestamp,
            txn_hash,
            miner_claim,
            claim_list_hash,
            block_reward,
            next_block_reward
        );

        let miner_signature = secret_key.sign_ecdsa(payload).to_string();

        BlockHeader {
            ref_hashes,
            round,
            block_seed,
            next_block_seed,
            block_height: last_block.header.block_height + 1,
            timestamp,
            txn_hash,
            miner_claim,
            claim_list_hash,
            block_reward,
            next_block_reward,
            miner_signature,
        }
    }

    pub fn get_payload(&self) -> Message {
        create_payload!(
            self.ref_hashes,
            self.block_seed,
            self.next_block_seed,
            self.block_height,
            self.timestamp,
            self.txn_hash,
            self.miner_claim,
            self.claim_list_hash,
            self.block_reward,
            self.next_block_reward,
            self.miner_signature
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
