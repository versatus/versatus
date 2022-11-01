// FEATURE TAG(S): Block Structure, Rewards
use std::{
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
    u32::MAX as u32MAX,
    u64::MAX as u64MAX, fmt::Display,
};

use bytebuffer::ByteBuffer;
use claim::claim::Claim;
use rand::Rng;
use reward::reward::{Reward, RewardState};
use secp256k1::{
    key::{PublicKey, SecretKey},
    Error, Message, Secp256k1, Signature,
};
use serde::{Deserialize, Serialize};
use sha256::digest_bytes;

use crate::block::Block;
use vrrb_vrf::{vrng::VRNG, vvrf::VVRF};

// TODO: Helper constants like the ones below should be in their own mod
pub const NANO: u128 = 1;
pub const MICRO: u128 = NANO * 1000;
pub const MILLI: u128 = MICRO * 1000;
pub const SECOND: u128 = MILLI * 1000;

#[derive(Debug)]
pub enum InvalidBlockHeader {
    InvalidSeedError,
    InvalidPubKeyError,
    InvalidMessageError,
}

impl Display for InvalidBlockHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvalidBlockHeader::InvalidSeedError => write!(f, "Invalid block seed generated"),
            InvalidBlockHeader::InvalidPubKeyError => write!(f, "Invalid public key"),
            InvalidBlockHeader::InvalidMessageError => write!(f, "Invalid message"),
        }
    }
}

impl std::error::Error for InvalidBlockHeader {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    // TODO: Rename block_nonce and next_block_nonce to block_seed and next_block_seed respectively
    // TODO: Replace tx hash with tx trie root
    // TODO: Replace claim hash with claim trie root
    // TODO: Add certificate field for validation certification.
    pub last_hash: String,
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
    pub fn genesis(
        nonce: u64,
        reward_state: &RewardState,
        claim: Claim,
        secret_key: String,
    ) -> Result<BlockHeader, InvalidBlockHeader> {
        //TODO: Replace rand::thread_rng() with VPRNG
        //TODO: Determine data fields to be used as message in VPRNG, must be
        // known/revealed within block but cannot be predictable or gameable.
        // Leading candidates are some combination of last_hash and last_block_seed
        let mut rng = rand::thread_rng();
        let last_hash = digest_bytes("Genesis_Last_Hash".as_bytes());
        let block_seed = nonce;
        // Range should remain the same.
        let next_block_seed: u64 = rng.gen_range(u32MAX as u64, u64MAX);

        //- previous block’s hash
        //- the previous block’s certification/validation signature (threshold)
        //- Current block miner signature
        

        let sk = VVRF::generate_secret_key();
        let mut vvrf = VVRF::new((last_hash).as_bytes(), sk);

        if VVRF::verify_seed(&mut vvrf).is_err() {
            return Err(InvalidBlockHeader::InvalidSeedError);
        }

        let mut random_number = vvrf.generate_u64();
        while random_number < u32MAX as u64 {
            random_number = vvrf.generate_u64();
        }

        if let Some()
        return Ok(random_number);







        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let txn_hash = digest_bytes("Genesis_Txn_Hash".as_bytes());
        let block_reward = Reward::genesis(Some(claim.address.clone()));
        //TODO: Replace reward state
        let next_block_reward = Reward::new(None, reward_state);
        let claim_map_hash: Option<String> = None;
        let neighbor_hash: Option<String> = None;
        let payload = format!(
            "{},{},{},{},{},{},{:?},{:?},{:?},{:?},{:?}",
            last_hash,
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

        let signature = BlockHeader::sign(&payload, secret_key).unwrap().to_string();

        BlockHeader {
            last_hash,
            block_seed,
            next_block_seed,
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
        reward_state: &RewardState,
        claim: Claim,
        txn_hash: String,
        claim_map_hash: Option<String>,
        neighbor_hash: Option<String>,
        secret_key: String,
    ) -> BlockHeader {
        //TODO: Replace rand::thread_rng() with VPRNG
        //TODO: Determine data fields to be used as message in VPRNG, must be
        // known/revealed within block but cannot be predictable or gameable.
        // Leading candidates are some combination of last_hash and last_block_seed
        let mut rng = rand::thread_rng();
        let last_hash = last_block.hash;
        let block_seed = last_block.header.next_block_seed.clone();
        let next_block_seed: u64 = rng.gen_range(0, u64MAX);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let mut block_reward = last_block.header.next_block_reward;
        block_reward.miner = Some(claim.clone().address);
        let next_block_reward = Reward::new(None, reward_state);
        let block_height = last_block.header.block_height + 1;
        let payload = format!(
            "{},{},{},{},{},{},{:?},{:?},{:?},{:?},{:?}",
            last_hash,
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

        let signature = BlockHeader::sign(&payload, secret_key).unwrap().to_string();

        BlockHeader {
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
            signature,
        }
    }

    pub fn sign(message: &str, secret_key: String) -> Result<Signature, Error> {
        let message_bytes = message.as_bytes().to_owned();
        let mut buffer = ByteBuffer::new();
        buffer.write_bytes(&message_bytes);
        while buffer.len() < 32 {
            buffer.write_u8(0);
        }

        let new_message = buffer.to_bytes();
        let message_hash = blake3::hash(&new_message);
        let message_hash = Message::from_slice(message_hash.as_bytes())?;
        let secp = Secp256k1::new();
        let sk = SecretKey::from_str(&secret_key).unwrap();
        let sig = secp.sign(&message_hash, &sk);
        Ok(sig)
    }

    // TODO: Additional Verification requirements
    pub fn verify(&self) -> Result<bool, Error> {
        let message_bytes = self.get_payload().as_bytes().to_vec();
        let signature = {
            if let Ok(signature) = Signature::from_str(&self.signature) {
                signature
            } else {
                return Err(Error::InvalidSignature);
            }
        };

        let pubkey = {
            if let Ok(pubkey) = PublicKey::from_str(&self.claim.pubkey) {
                pubkey
            } else {
                return Err(Error::InvalidPublicKey);
            }
        };

        let mut buffer = ByteBuffer::new();
        buffer.write_bytes(&message_bytes);
        while buffer.len() < 32 {
            buffer.write_u8(0);
        }
        let new_message = buffer.to_bytes();
        let message_hash = blake3::hash(&new_message);
        let message_hash = Message::from_slice(message_hash.as_bytes())?;
        let secp = Secp256k1::new();
        let valid = secp.verify(&message_hash, &signature, &pubkey);

        match valid {
            Ok(()) => Ok(true),
            _ => Err(Error::IncorrectSignature),
        }
    }

    pub fn get_payload(&self) -> String {
        format!(
            "{},{},{},{},{},{},{:?},{:?},{:?},{:?},{:?}",
            self.last_hash,
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
        )
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_string().as_bytes().to_vec()
    }

    pub fn from_bytes(data: &[u8]) -> BlockHeader {
        serde_json::from_slice(data).unwrap()
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn from_str(data: &str) -> BlockHeader {
        serde_json::from_str(data).unwrap()
    }
}
