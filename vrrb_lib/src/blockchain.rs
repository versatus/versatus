use crate::block::Block;
use crate::fields::GettableFields;
use crate::header::BlockHeader;
use commands::command::Command;
use messages::message_types::MessageType;
use crate::reward::RewardState;
use crate::state::NetworkState;
use crate::verifiable::Verifiable;
use pickledb::{PickleDb, PickleDbDumpPolicy, SerializationMethod};
use ritelinked::LinkedHashMap;
use serde::{Deserialize, Serialize};
use std::collections::LinkedList;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blockchain {
    pub genesis: Option<Block>,
    pub child: Option<Block>,
    pub parent: Option<Block>,
    pub chain: LinkedList<BlockHeader>,
    pub chain_db: String, // Path to the chain database.
    pub block_cache: LinkedHashMap<String, Block>,
    pub future_blocks: LinkedHashMap<String, Block>,
    pub invalid: LinkedHashMap<String, Block>,
    pub updating_state: bool,
    pub state_update_cache: LinkedHashMap<u128, LinkedHashMap<u128, Vec<u8>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InvalidBlockErrorReason {
    BlockOutOfSequence,
    InvalidClaim,
    InvalidLastHash,
    InvalidStateHash,
    InvalidBlockHeight,
    InvalidBlockNonce,
    InvalidBlockReward,
    InvalidTxns,
    InvalidClaimPointers,
    General,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidBlockError {
    pub details: InvalidBlockErrorReason,
}

impl Blockchain {
    pub fn new(path: &str) -> Blockchain {
        Blockchain {
            genesis: None,
            child: None,
            parent: None,
            chain: LinkedList::new(),
            chain_db: path.to_string(),
            block_cache: LinkedHashMap::new(),
            future_blocks: LinkedHashMap::new(),
            invalid: LinkedHashMap::new(),
            updating_state: false,
            state_update_cache: LinkedHashMap::new(),
        }
    }

    pub fn check_next_block_height(&self, block: &Block) -> bool {
        if let Some(_) = self.genesis.as_ref() {
            if let Some(child) = self.child.as_ref() {
                if child.header.block_height + 1 != block.header.block_height {
                    return false;
                } else {
                    return true;
                }
            } else {
                if block.header.block_height != 1 {
                    return false;
                } else {
                    return true;
                }
            }
        } else {
            if block.header.block_height != 0 {
                return false;
            } else {
                return true;
            }
        }
    }

    pub fn get_chain_db(&self) -> PickleDb {
        match PickleDb::load_bin(self.chain_db.clone(), PickleDbDumpPolicy::DumpUponRequest) {
            Ok(nst) => nst,
            Err(_) => PickleDb::new(
                self.chain_db.clone(),
                PickleDbDumpPolicy::DumpUponRequest,
                SerializationMethod::Bin,
            ),
        }
    }

    pub fn clone_chain_db(&self) -> PickleDb {
        let db = self.get_chain_db();
        let keys = db.get_all();

        let mut cloned_db = PickleDb::new(
            format!("temp_{}.db", self.chain_db.clone()),
            PickleDbDumpPolicy::NeverDump,
            SerializationMethod::Bin,
        );

        keys.iter().for_each(|k| {
            let block = db.get::<Block>(k);
            if let Some(block) = block {
                if let Err(e) = cloned_db.set(k, &block) {
                    println!(
                        "Error setting block with last_hash {} to cloned_db: {:?}",
                        k, e
                    );
                }
            }
        });

        drop(db);
        cloned_db
    }

    pub fn chain_db_to_string(&self) -> String {
        let db = self.clone_chain_db();
        let mut db_map = LinkedHashMap::new();
        let keys = db.get_all();

        for key in keys.iter() {
            let value = db.get::<Block>(&key).unwrap();
            let k = key.clone();
            db_map.insert(k, value);
        }

        serde_json::to_string(&db_map).unwrap()
    }

    pub fn chain_db_to_bytes(&self) -> Vec<u8> {
        self.chain_db_to_string().as_bytes().to_vec()
    }

    pub fn chain_db_from_bytes(&self, data: &[u8]) -> PickleDb {
        let db_map = serde_json::from_slice::<LinkedHashMap<String, Block>>(data).unwrap();

        let mut db = PickleDb::new_bin(self.clone().chain_db, PickleDbDumpPolicy::DumpUponRequest);

        db_map.iter().for_each(|(k, v)| {
            if let Err(e) = db.set(&k, &v) {
                println!("Error setting block in database: {:?}", e);
            };
        });

        db
    }

    pub fn dump(&self, block: &Block) -> Result<(), Box<dyn Error>> {
        let mut db = self.get_chain_db();
        if let Err(e) = db.set(&block.header.last_hash, block) {
            return Err(Box::new(e));
        }

        if let Err(e) = db.dump() {
            return Err(Box::new(e));
        }

        Ok(())
    }

    pub fn get_block(&self, last_hash: &str) -> Option<Block> {
        let db = self.get_chain_db();
        db.get::<Block>(last_hash)
    }

    pub fn process_block(
        &mut self,
        network_state: &NetworkState,
        reward_state: &RewardState,
        block: &Block,
    ) -> Result<(), InvalidBlockError> {
        if let Some(genesis_block) = &self.genesis {
            if let Some(last_block) = &self.child {
                if let Err(e) = block.valid_block(&last_block, network_state, reward_state) {
                    self.future_blocks
                        .insert(block.clone().header.last_hash, block.clone());
                    return Err(e);
                } else {
                    self.parent = self.child.clone();
                    self.child = Some(block.clone());
                    self.chain.push_back(block.header.clone());
                    if self.block_cache.len() == 100 {
                        self.block_cache.pop_back();
                        self.block_cache.insert(block.hash.clone(), block.clone());
                    }

                    if let Err(e) = self.dump(&block) {
                        println!("Error dumping block to chain db: {:?}", e);
                    };

                    return Ok(());
                }
            } else {
                if let Err(e) = block.valid_block(&genesis_block, network_state, reward_state) {
                    return Err(e);
                } else {
                    self.child = Some(block.clone());
                    self.chain.push_back(block.header.clone());
                    if let Err(e) = self.dump(&block) {
                        println!("Error dumping block to chain db: {:?}", e);
                    };
                    Ok(())
                }
            }
        } else {
            // check that this is a valid genesis block.
            if block.header.block_height == 0 {
                if block.valid_genesis(network_state, reward_state) {
                    self.genesis = Some(block.clone());
                    self.child = Some(block.clone());
                    self.block_cache.insert(block.hash.clone(), block.clone());
                    self.chain.push_back(block.header.clone());
                    if let Err(e) = self.dump(&block) {
                        println!("Error dumping block to chain db: {:?}", e);
                    };
                    Ok(())
                } else {
                    self.invalid.insert(block.hash.clone(), block.clone());
                    Err(InvalidBlockError {
                        details: InvalidBlockErrorReason::General,
                    })
                }
            } else {
                // request genesis block.
                self.future_blocks
                    .insert(block.clone().header.last_hash, block.clone());
                Err(InvalidBlockError {
                    details: InvalidBlockErrorReason::BlockOutOfSequence,
                })
            }
        }
    }

    pub fn stash_future_blocks(&mut self, block: &Block) {
        self.future_blocks
            .insert(block.clone().header.last_hash, block.clone());
    }

    pub fn send_invalid_block_message(
        &self,
        block: &Block,
        reason: InvalidBlockErrorReason,
        miner_id: String,
        sender_id: String,
        swarm_sender: tokio::sync::mpsc::UnboundedSender<Command>,
    ) {
        let message = MessageType::InvalidBlockMessage {
            block_height: block.clone().header.block_height,
            reason: reason.as_bytes(),
            miner_id,
            sender_id,
        };

        if let Err(e) = swarm_sender.send(Command::SendMessage(message.as_bytes())) {
            println!(
                "Error sending InvalidBlockMessage InvalidBlockHeight to swarm sender: {:?}",
                e
            );
        }
    }

    pub fn send_missing_blocks_message(
        &self,
        block: &Block,
        sender_id: String,
        swarm_sender: tokio::sync::mpsc::UnboundedSender<Command>,
    ) {
        let missing_blocks: Vec<u128> =
            (self.chain.len() as u128 - 1u128..block.clone().header.block_height).collect();

        let message = MessageType::NeedBlocksMessage {
            blocks_needed: missing_blocks,
            sender_id,
        };

        if let Err(e) = swarm_sender.send(Command::SendMessage(message.as_bytes())) {
            println!("Error sending NeedBlocksMessage to swarm sender: {:?}", e);
        }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_string().as_bytes().to_vec()
    }

    pub fn from_bytes(data: &[u8]) -> Blockchain {
        serde_json::from_slice::<Blockchain>(&data).unwrap()
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn from_string(data: &str) -> Blockchain {
        serde_json::from_str(data).unwrap()
    }

    pub fn get_field_names(&self) -> Vec<String> {
        return vec![
            "genesis".to_string(),
            "child".to_string(),
            "parent".to_string(),
            "chain".to_string(),
            "chain_db".to_string(),
            "block_cache".to_string(),
            "future_blocks".to_string(),
            "invalid".to_string(),
            "updating_state".to_string(),
            "state_update_cache".to_string(),
        ];
    }
}

impl InvalidBlockErrorReason {
    pub fn to_str(&self) -> &str {
        match self {
            Self::BlockOutOfSequence => "block out of sequence",
            Self::General => "general invalid block",
            Self::InvalidBlockHeight => "invalid block height",
            Self::InvalidClaim => "invalid claim",
            Self::InvalidLastHash => "invalid last hash",
            Self::InvalidStateHash => "invalid state hash",
            Self::InvalidBlockNonce => "invalid block nonce",
            Self::InvalidBlockReward => "invalid block reward",
            Self::InvalidTxns => "invalid txns in block",
            Self::InvalidClaimPointers => "invalid claim pointers",
        }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        serde_json::to_string(self).unwrap().as_bytes().to_vec()
    }
}

impl fmt::Display for Blockchain {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Blockchain")
    }
}

impl Error for Blockchain {}

impl fmt::Display for InvalidBlockError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for InvalidBlockError {
    fn description(&self) -> &str {
        &self.details.to_str()
    }
}

impl Error for InvalidBlockErrorReason {
    fn description(&self) -> &str {
        &self.to_str()
    }
}

impl fmt::Display for InvalidBlockErrorReason {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InvalidBlockHeight => {
                write!(f, "invalid block height")
            }
            Self::InvalidClaim => {
                write!(f, "invalid claim")
            }
            Self::InvalidLastHash => {
                write!(f, "invalid last hash")
            }
            Self::InvalidStateHash => {
                write!(f, "invalid state hash")
            }
            Self::BlockOutOfSequence => {
                write!(f, "block out of sequence")
            }
            Self::InvalidBlockNonce => {
                write!(f, "invalid block nonce")
            }
            Self::InvalidBlockReward => {
                write!(f, "invalid block reward")
            }
            Self::InvalidTxns => {
                write!(f, "invalid txns in block")
            }
            Self::InvalidClaimPointers => {
                write!(f, "invalid claim pointers")
            }
            Self::General => {
                write!(f, "general invalid block error")
            }
        }
    }
}

impl GettableFields for Blockchain {
    fn get_field(&self, field: &str) -> Option<String> {
        match field {
            "genesis" => {
                if let Some(genesis) = self.genesis.clone() {
                    return Some(genesis.to_string());
                }
                return None;
            }
            "child" => {
                if let Some(child) = self.child.clone() {
                    return Some(child.to_string());
                }
                return None;
            }
            "parent" => {
                if let Some(parent) = self.parent.clone() {
                    return Some(parent.to_string());
                }
                return None;
            }
            "chain" => return Some(serde_json::to_string(&self.chain).unwrap()),
            "chain_db" => Some(self.chain_db.clone()),
            "block_cache" => return Some(serde_json::to_string(&self.block_cache).unwrap()),
            "future_blocks" => return Some(serde_json::to_string(&self.future_blocks).unwrap()),
            "invalid" => return Some(serde_json::to_string(&self.invalid).unwrap()),
            "updating_state" => return Some(format!("{}", self.updating_state)),
            "state_update_cache" => {
                return Some(serde_json::to_string(&self.state_update_cache).unwrap())
            }
            _ => None,
        }
    }
}
