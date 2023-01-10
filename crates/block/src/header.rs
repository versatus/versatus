// FEATURE TAG(S): Block Structure, Rewards
use std::{
    time::{SystemTime, UNIX_EPOCH},
    u32::MAX as u32MAX,
};

use bytebuffer::ByteBuffer;
use primitives::types::RawSignature;
use primitives::SerializedSecretKey as SecretKeyBytes;
use reward::reward::Reward;
use serde::{Deserialize, Serialize};
use sha256::digest;
use vrrb_core::{claim::Claim, keypair::KeyPair};
use vrrb_vrf::{vrng::VRNG, vvrf::VVRF};

use crate::{block::Block, NextEpochAdjustment, invalid::InvalidBlockErrorReason, invalid::InvalidBlockError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    // TODO: Rename block_seed and next_block_seed to block_seed and next_block_seed respectively
    // TODO: Replace tx hash with tx trie root
    // TODO: Replace claim hash with claim trie root
    // TODO: Add certificate field for validation certification.
    pub last_hash: Vec<u8>,
    pub block_seed: u64,
    pub next_block_seed: u64,
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

    //TODO: miners needs to wait on threshold signature before passing to this fxn
    pub fn genesis(
        seed: u64,
        claim: Claim,
        secret_key: Vec<u8>,
        miner: Option<String>,
        threshold_signature: RawSignature
    ) -> Result<BlockHeader, InvalidBlockErrorReason> {
        //TODO: Replace rand::thread_rng() with VPRNG
        //TODO: Determine data fields to be used as message in VPRNG, must be
        // known/revealed within block but cannot be predictable or gameable.
        // Leading candidates are some combination of last_hash and last_block_seed
        //let mut rng = rand::thread_rng();
        let last_hash = "Genesis_Last_Hash".as_bytes().to_vec();
        let block_seed = seed;
        // Range should remain the same.

        let next_block_seed = match Self::generate_next_block_seed(last_hash.clone(), threshold_signature.clone()) {
            Ok(next_block_seed) => next_block_seed,
            Err(e) => return Err(e),
        };

        let timestamp: u128;
        if let Ok(temp_timestamp) = SystemTime::now().duration_since(UNIX_EPOCH){
            timestamp = temp_timestamp.as_nanos();
        }
        else{
            return Err(InvalidBlockErrorReason::InvalidBlockHeader);
        }
        let txn_hash = digest("Genesis_Txn_Hash".as_bytes());
        let block_reward = Reward::genesis(Some(claim.address.clone()));
        //TODO: Replace reward state
        let next_block_reward = Reward::genesis(miner);
        let claim_map_hash: Option<String> = None;
        let neighbor_hash: Option<String> = None;
        let mut payload = String::new();
        if let Ok(str_last_hash) = String::from_utf8(last_hash.clone()){
            payload = format!(
                "{},{},{},{},{},{},{:?},{:?},{:?},{:?},{:?}",
                str_last_hash,
                block_seed,
                next_block_seed,
                0,
                timestamp,
                txn_hash,
                claim,
                claim_map_hash,
                block_reward,
                next_block_reward,
                neighbor_hash,
            );
        }

        let payload_bytes = payload.as_bytes();
        
        if let Ok(signature) =  KeyPair::ecdsa_signature(payload_bytes, &secret_key){
            Ok(BlockHeader {
                last_hash,
                block_seed,
                next_block_seed,
                block_height: 0,
                timestamp,
                txn_hash,
                claim,
                claim_map_hash,
                block_reward,
                next_block_reward,
                neighbor_hash: None,
                signature: signature.to_string(),
            })   
        } else {
            Err(InvalidBlockErrorReason::InvalidBlockHeader)
        }
    }

    pub fn new(
        last_block: Block,
        reward: &mut Reward,
        claim: Claim,
        txn_hash: String,
        claim_map_hash: Option<String>,
        neighbor_hash: Option<String>,
        secret_key: SecretKeyBytes,
        epoch_change: bool,
        adjustment_next_epoch: NextEpochAdjustment,
        wrapped_threshold_signature: Option<RawSignature>
    ) -> Result<BlockHeader, InvalidBlockErrorReason> {
        //TODO: Replace rand::thread_rng() with VPRNG
        //TODO: Determine data fields to be used as message in VPRNG, must be
        // known/revealed within block but cannot be predictable or gameable.
        // Leading candidates are some combination of last_hash and last_block_seed
        //let mut rng = rand::thread_rng();
        let last_hash = last_block.hash;
        let block_seed = last_block.header.next_block_seed;

        let threshold_signature = wrapped_threshold_signature.ok_or(InvalidBlockErrorReason::InvalidBlockHeader);

        let next_block_seed = match Self::generate_next_block_seed(last_hash.clone(), threshold_signature.clone()?) {
            Ok(next_block_seed) => next_block_seed,
            Err(e) => return Err(e),
        };
       
        let timestamp: u128;
        if let Ok(temp_timestamp) = SystemTime::now().duration_since(UNIX_EPOCH){
            timestamp = temp_timestamp.as_nanos();
        } else {
            return Err(InvalidBlockErrorReason::InvalidBlockHeader);
        }
        let mut block_reward = last_block.header.next_block_reward;
        block_reward.miner = Some(claim.clone().address);

        let mut next_block_reward = reward.clone();
        if epoch_change {
            reward.new_epoch(adjustment_next_epoch);
            next_block_reward = reward.clone();
        }
        let block_height = last_block.header.block_height + 1;

        let mut payload = String::new();

        if let Ok(str_last_hash) =  String::from_utf8(last_hash.clone()){
            payload = format!(
                "{},{},{},{},{},{},{:?},{:?},{:?},{:?},{:?}",
                str_last_hash,
                block_seed,
                next_block_seed,
                block_height,
                timestamp,
                txn_hash,
                claim,
                claim_map_hash,
                block_reward,
                next_block_reward,
                neighbor_hash,
            );
        }
        
        let payload_bytes = payload.as_bytes();
        
        if let Ok(signature) =  KeyPair::ecdsa_signature(payload_bytes, &secret_key){
            Ok(BlockHeader {
                last_hash,
                block_seed,
                next_block_seed,
                block_height: last_block.header.block_height + 1,
                timestamp,
                txn_hash,
                claim,
                claim_map_hash,
                block_reward,
                next_block_reward,
                neighbor_hash: None,
                signature: signature.to_string(),
            })   
        } else {
            Err(InvalidBlockErrorReason::InvalidBlockHeader)
        }
    }

    pub fn get_payload(&self) -> String {
        if let Ok(str_last_hash) = String::from_utf8(self.last_hash.clone()){
            return format!(
                "{},{},{},{},{},{},{:?},{:?},{:?},{:?},{:?}",
                str_last_hash,
                self.block_seed,
                self.next_block_seed,
                self.block_height,
                self.timestamp,
                self.txn_hash,
                self.claim,
                self.claim_map_hash,
                self.block_reward,
                self.next_block_reward,
                self.neighbor_hash,
            );
        }
        return String::new();
    }

    //TODO Option wrapper removed from threshiold_signature as waiting will be required before it can be passed in 
    pub fn generate_next_block_seed(last_hash: Vec<u8>, threshold_sig: RawSignature) -> Result<u64, InvalidBlockErrorReason>{
        let sk = KeyPair::random();
        let msg: Vec<u8> = last_hash.iter().cloned().chain(threshold_sig.iter().cloned()).collect();
        let mut vvrf = VVRF::new(&msg, &sk);

        if VVRF::verify_seed(&mut vvrf).is_err() {
            return Err(InvalidBlockErrorReason::InvalidBlockHeader);
        }

        let mut random_number = vvrf.generate_u64();
        while random_number < u32MAX as u64 {
            random_number = vvrf.generate_u64();
        }

        return Ok(random_number);
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
