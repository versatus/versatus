use std::fmt::Debug;
// FEATURE TAG(S): Block Structure, Rewards
use chrono;
use primitives::{Epoch, SecretKey};
use reward::reward::Reward;
use secp256k1::{
    hashes::{sha256 as s256, Hash},
    Message,
};
use serde::{Deserialize, Serialize};
use utils::{create_payload, hash_data};
use vrrb_core::claim::Claim;
use vrrb_vrf::{vrng::VRNG, vvrf::VVRF};

use crate::{block::Block, InnerBlock, NextEpochAdjustment};

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct BlockHeader {
    // TODO: Replace tx hash with tx trie root???
    // TODO: Replace claim hash with claim trie root???
    pub ref_hashes: Vec<String>,
    pub epoch: Epoch,
    pub round: u128,
    pub block_seed: u64,
    pub next_block_seed: u64,
    pub block_height: u128,
    pub timestamp: i64,
    pub txn_hash: String,
    pub miner_claim: Claim,
    pub claim_list_hash: String,
    pub block_reward: Reward,
    pub next_block_reward: Reward,
    pub miner_signature: String,
}

impl BlockHeader {
    //TODO: miners needs to wait on threshold signature before passing to this fxn
    pub fn genesis(
        seed: u64,
        round: u128,
        epoch: Epoch,
        miner_claim: Claim,
        secret_key: SecretKey,
        claim_list_hash: String,
    ) -> BlockHeader {
        //TODO: Determine data fields to be used as message in VPRNG, must be
        // known/revealed within block but cannot be predictable or gameable.
        // Leading candidates are some combination of last_hash and last_block_seed
        let ref_hashes = vec![hex::encode(hash_data!("Genesis_Ref_hash".to_string()))];
        let ref_hashes_hash = hex::encode(hash_data!(ref_hashes));
        let genesis_last_hash = hex::encode(hash_data!("Genesis_Last_Hash".to_string()));
        let message = {
            hex::encode(hash_data!(ref_hashes_hash, genesis_last_hash))
                .as_bytes()
                .to_vec()
        };

        let mut vrf = VVRF::new(&message, secret_key.secret_bytes().as_ref());

        let next_block_seed = vrf.generate_u64_in_range(u32::MAX as u64, u64::MAX);

        let timestamp = chrono::Utc::now().timestamp();
        let txn_hash = hex::encode(hash_data!("Genesis_Txn_Hash".to_string()));
        let block_reward = Reward::genesis(Some(miner_claim.address.to_string()));
        let block_height = 0;
        let next_block_reward = Reward::default();

        let payload = create_payload!(
            ref_hashes,
            round,
            epoch,
            seed,
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
            epoch,
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
        last_block: Block,
        ref_hashes: Vec<String>,
        miner_claim: Claim,
        secret_key: SecretKey,
        txn_hash: String,
        claim_list_hash: String,
        adjustment_next_epoch: NextEpochAdjustment,
    ) -> Option<BlockHeader> {
        // Get the last block
        let last_block: &dyn InnerBlock<Header = BlockHeader, RewardType = Reward> = {
            match last_block {
                Block::Convergence { ref block } => block,
                Block::Genesis { ref block } => block,
                _ => return None,
            }
        };

        // Get the current block seed, which is last_block.next_block_seed;
        let block_seed = last_block.get_next_block_seed();

        // Get block height
        let block_height = last_block.get_header().block_height + 1;

        // get the message; TODO: replace ref_hashes with
        // last_block.certificate
        let message = {
            let hash = hex::encode(hash_data!(last_block.get_hash(), ref_hashes));
            hash.as_bytes().to_vec()
        };

        let sk_bytes = &secret_key.secret_bytes();

        // Generate next_block_seed
        let mut vrf = VVRF::new(&message, sk_bytes);
        let next_block_seed = vrf.generate_u64_in_range(u32::MAX as u64, u64::MAX);

        // generate timestamp
        let timestamp = chrono::Utc::now().timestamp();

        // Get current block reward, which is last_block.next_block_reward
        let mut block_reward = last_block.get_next_block_reward();
        block_reward.current_block = block_height;

        // Create the next block reward, which is a clone of the current
        // reward, unless there's an epoch change
        let next_block_reward = block_reward.generate_next_reward(adjustment_next_epoch);

        // Append the miner to the current block reward
        block_reward.miner = Some(miner_claim.address.to_string());

        // Get current epoch which is the same as last epoch unless it's an
        // epoch change block.
        let epoch = last_block.get_header().epoch;
        // Get the reward for current block which is last_block.round + 1
        let round = last_block.get_header().round + 1;

        let payload = create_payload!(
            ref_hashes,
            round,
            epoch,
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

        let block_header = BlockHeader {
            ref_hashes,
            round,
            epoch,
            block_seed,
            next_block_seed,
            block_height: last_block.get_header().block_height + 1,
            timestamp,
            txn_hash,
            miner_claim,
            claim_list_hash,
            block_reward,
            next_block_reward,
            miner_signature,
        };

        Some(block_header)
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
