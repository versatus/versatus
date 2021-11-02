use crate::block::Block;
use crate::claim::Claim;
use crate::reward::{Reward, RewardState};
use bytebuffer::ByteBuffer;
use rand::Rng;
use secp256k1::Error;
use secp256k1::{
    key::{PublicKey, SecretKey},
    Signature,
};
use secp256k1::{Message, Secp256k1};
use serde::{Deserialize, Serialize};
use sha256::digest_bytes;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use std::u32::MAX as u32MAX;
use std::u64::MAX as u64MAX;

pub const NANO: u128 = 1;
pub const MICRO: u128 = NANO * 1000;
pub const MILLI: u128 = MICRO * 1000;
pub const SECOND: u128 = MILLI * 1000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
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
        reward_state: &RewardState,
        claim: Claim,
        secret_key: String,
    ) -> BlockHeader {
        let mut rng = rand::thread_rng();
        let last_hash = digest_bytes("Genesis_Last_Hash".as_bytes());
        let block_nonce = nonce;
        let next_block_nonce: u64 = rng.gen_range(u32MAX as u64, u64MAX);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let txn_hash = digest_bytes("Genesis_Txn_Hash".as_bytes());
        let block_reward = Reward::genesis(Some(claim.address.clone()));
        let next_block_reward = Reward::new(None, reward_state);
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

        let signature = BlockHeader::sign(&payload, secret_key).unwrap().to_string();

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
        reward_state: &RewardState,
        claim: Claim,
        txn_hash: String,
        claim_map_hash: Option<String>,
        neighbor_hash: Option<String>,
        secret_key: String,
    ) -> BlockHeader {
        let mut rng = rand::thread_rng();
        let last_hash = last_block.hash;
        let block_nonce = last_block.header.next_block_nonce.clone();
        let next_block_nonce: u64 = rng.gen_range(0, u64MAX);
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

        let signature = BlockHeader::sign(&payload, secret_key).unwrap().to_string();

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

    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
    pub fn from_str(data: &str) -> BlockHeader {
        serde_json::from_str(data).unwrap()
    }
}
