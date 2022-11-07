// Feature Tags: Block Structure, State Syncing, Block Validation, Block
// Confirmation
use std::{
    collections::{HashSet, LinkedList},
    error::Error,
    fmt,
    net::SocketAddr,
};

use block::{
    block::Block,
    header::BlockHeader,
    invalid::{InvalidBlockError, InvalidBlockErrorReason},
};
use commands::command::ComponentTypes;
use log::info;
use messages::message_types::MessageType;
use pickledb::{PickleDb, PickleDbDumpPolicy, SerializationMethod};
use reward::reward::RewardState;
use ritelinked::LinkedHashMap;
use serde::{Deserialize, Serialize};
use state::state::NetworkState;
use udp2p::{
    gossip::protocol::GossipMessage,
    protocol::protocol::{Header, Message, MessageKey},
    utils::utils::{timestamp_now, ByteRep},
};
use verifiable::verifiable::Verifiable;
use vrrb_lib::fields::GettableFields;

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
    pub components_received: HashSet<ComponentTypes>,
    pub updating_state: bool,
    pub processing_backlog: bool,
    pub started_updating: Option<u128>,
    pub state_update_cache: LinkedHashMap<u128, LinkedHashMap<u128, Vec<u8>>>,
}

impl Blockchain {
    pub fn new(path: &str) -> Blockchain {
        Blockchain {
            genesis: None,
            child: None,
            parent: None,
            // TODO: Debate whether Replace??
            chain: LinkedList::new(),
            // TODO: Make optional
            chain_db: path.to_string(),
            block_cache: LinkedHashMap::new(),
            future_blocks: LinkedHashMap::new(),
            invalid: LinkedHashMap::new(),
            components_received: HashSet::new(),
            updating_state: false,
            processing_backlog: false,
            started_updating: None,
            state_update_cache: LinkedHashMap::new(),
        }
    }

    /// Checks if the next block height is valid, i.e. +1 as compared to
    /// previous block.
    pub fn check_next_block_height(&self, block: &Block) -> bool {
        // Check if there is a genesis block
        if self.genesis.is_some() {
            // If so, check if there is a child block
            if let Some(child) = self.child.as_ref() {
                // If so check if the block height is equal to last block's height + 1
                if child.header.block_height + 1 != block.header.block_height {
                    // If not, then return false (invalid height)
                    return false;
                }
            } else {
                // otherwise check if the block height is one
                // if not, return false (invalid height)
                if block.header.block_height != 1 {
                    return false;
                }
            }
        } else {
            // If there is no genesis block, then check if the block height is 0
            // if not, return false
            if block.header.block_height != 0 {
                return false;
            }
        }

        true
    }

    /// Loads the chain from binary and returns a PickleDB instance
    pub fn get_chain_db(&self) -> PickleDb {
        match PickleDb::load(
            self.chain_db.clone(),
            PickleDbDumpPolicy::DumpUponRequest,
            SerializationMethod::Bin,
        ) {
            Ok(nst) => nst,
            Err(_) => PickleDb::new(
                self.chain_db.clone(),
                PickleDbDumpPolicy::DumpUponRequest,
                SerializationMethod::Bin,
            ),
        }
    }

    /// Creates a clone of a PickleDB Instance containing chain data.
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

    /// Serializes the Chain Database into a string
    pub fn chain_db_to_string(&self) -> String {
        let db = self.clone_chain_db();
        let mut db_map = LinkedHashMap::new();
        let keys = db.get_all();

        for key in keys.iter() {
            let value = db.get::<Block>(key).unwrap();
            let k = key.clone();
            db_map.insert(k, value);
        }

        serde_json::to_string(&db_map).unwrap()
    }

    /// Serializes the Chain Database into a vector of bytes of any size
    pub fn chain_db_to_bytes(&self) -> Vec<u8> {
        self.chain_db_to_string().as_bytes().to_vec()
    }

    /// Deserializes a slice of bytes into a PickleDB Instance
    pub fn chain_db_from_bytes(&self, data: &[u8]) -> PickleDb {
        let db_map = serde_json::from_slice::<LinkedHashMap<String, Block>>(data).unwrap();

        let mut db = PickleDb::new(
            self.clone().chain_db,
            PickleDbDumpPolicy::DumpUponRequest,
            SerializationMethod::Bin,
        );

        db_map.iter().for_each(|(k, v)| {
            if let Err(e) = db.set(k, &v) {
                println!("Error setting block in database: {:?}", e);
            };
        });

        db
    }

    /// Dumps data to a PickleDB Instance
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

    /// Retrieves a block based on the `last_hash` field. Returns an option
    /// (Some(Block) if the block exists in the db, None if it does not)
    pub fn get_block(&self, last_hash: &str) -> Option<Block> {
        let db = self.get_chain_db();
        db.get::<Block>(last_hash)
    }

    /// Processes a block and returns either a result (Ok(()) if the block is
    /// valid, InvalidBlockError if not)
    pub fn process_block(
        &mut self,
        network_state: &NetworkState,
        reward_state: &RewardState,
        block: &Block,
    ) -> Result<(), InvalidBlockError> {
        if let Err(e) = self.check_block_sequence(block) {
            return Err(e);
        }
        if let Some(genesis_block) = &self.genesis {
            if let Some(last_block) = &self.child {
                if let Err(e) = block.valid(
                    last_block,
                    &(network_state.to_owned(), reward_state.to_owned()),
                ) {
                    self.future_blocks
                        .insert(block.clone().header.last_hash, block.clone());
                    Err(e)
                } else {
                    self.parent = self.child.clone();
                    self.child = Some(block.clone());
                    self.chain.push_back(block.header.clone());
                    if self.block_cache.len() == 100 {
                        self.block_cache.pop_back();
                        self.block_cache.insert(block.hash.clone(), block.clone());
                    }

                    if let Err(e) = self.dump(block) {
                        println!("Error dumping block to chain db: {:?}", e);
                    };

                    Ok(())
                }
            } else if let Err(e) = block.valid(
                genesis_block,
                &(network_state.to_owned(), reward_state.to_owned()),
            ) {
                Err(e)
            } else {
                self.child = Some(block.clone());
                self.chain.push_back(block.header.clone());
                if let Err(e) = self.dump(block) {
                    println!("Error dumping block to chain db: {:?}", e);
                };
                Ok(())
            }
        } else {
            // check that this is a valid genesis block.
            if block.header.block_height == 0 {
                if let Ok(true) =
                    block.valid_genesis(&(network_state.to_owned(), reward_state.to_owned()))
                {
                    self.genesis = Some(block.clone());
                    self.child = Some(block.clone());
                    self.block_cache.insert(block.hash.clone(), block.clone());
                    self.chain.push_back(block.header.clone());
                    if let Err(e) = self.dump(block) {
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

    // TODO: Discuss whether some of, or everything from here down should be moved
    // to a separate module for: a. readability
    // b. efficiency
    // c. to better organize similar functionality

    /// Checks whether the block is in sequence or not.
    pub fn check_block_sequence(&self, block: &Block) -> Result<bool, InvalidBlockError> {
        if self.genesis.is_some() {
            if let Some(child) = self.child.clone() {
                let next_height = child.header.block_height + 1;

                match block.header.block_height.cmp(&next_height) {
                    std::cmp::Ordering::Less => Err(InvalidBlockError {
                        details: InvalidBlockErrorReason::NotTallestChain,
                    }),
                    std::cmp::Ordering::Equal => Ok(true),
                    std::cmp::Ordering::Greater =>
                    //I'm missing blocks return BlockOutOfSequence error
                    {
                        Err(InvalidBlockError {
                            details: InvalidBlockErrorReason::BlockOutOfSequence,
                        })
                    },
                }
            } else {
                match block.header.block_height.cmp(&1) {
                    std::cmp::Ordering::Less => Err(InvalidBlockError {
                        details: InvalidBlockErrorReason::NotTallestChain,
                    }),
                    std::cmp::Ordering::Equal => Ok(true),
                    std::cmp::Ordering::Greater => Err(InvalidBlockError {
                        details: InvalidBlockErrorReason::BlockOutOfSequence,
                    }),
                }
            }
        } else if block.header.block_height != 0 {
            Err(InvalidBlockError {
                details: InvalidBlockErrorReason::BlockOutOfSequence,
            })
        } else {
            Ok(true)
        }
    }

    /// Puts blocks into an ordered map to process later in the event that the
    /// chain is updating the state.
    pub fn stash_future_blocks(&mut self, block: &Block) {
        self.future_blocks
            .insert(block.clone().header.last_hash, block.clone());
    }

    /// Creates and sends (to transport layer channel for sending to
    /// network/miner) a message in the event of an invalid block
    /// to inform the miner that they proposed an invalid block.
    pub fn send_invalid_block_message(
        &self,
        block: &Block,
        reason: InvalidBlockErrorReason,
        miner_id: String,
        sender_id: String,
        gossip_tx: std::sync::mpsc::Sender<(SocketAddr, Message)>,
        src: SocketAddr,
    ) {
        let message = MessageType::InvalidBlockMessage {
            block_height: block.clone().header.block_height,
            reason: reason.as_bytes(),
            miner_id,
            sender_id,
        };
        let msg_id = MessageKey::rand();
        let gossip_msg = GossipMessage {
            id: msg_id.inner(),
            data: message.as_bytes(),
            sender: src,
        };
        let head = Header::Gossip;
        let msg = Message {
            head,
            msg: gossip_msg.as_bytes().unwrap(),
        };

        if let Err(e) = gossip_tx.send((src, msg)) {
            println!(
                "Error sending InvalidBlockMessage InvalidBlockHeight to swarm sender: {:?}",
                e
            );
        }
    }

    /// Checks if all state core components have been received
    /// Core components include:
    ///     Genesis Block
    ///     Child (Last) Block
    ///     Parent (Previous block to child) Block
    ///     The current state of the network
    ///     The current network ledger
    pub fn received_core_components(&self) -> bool {
        self.components_received.contains(&ComponentTypes::Genesis)
            && self.components_received.contains(&ComponentTypes::Child)
            && self.components_received.contains(&ComponentTypes::Parent)
            && self
                .components_received
                .contains(&ComponentTypes::NetworkState)
            && self.components_received.contains(&ComponentTypes::Ledger)
    }

    /// Checks how long since the request was sent for state update
    pub fn check_time_since_update_request(&self) -> Option<u128> {
        let now = timestamp_now();
        if let Some(time) = self.started_updating {
            let diff = now.checked_sub(time);
            info!("Time in nanos since last update: {:?}", diff);
            diff
        } else {
            None
        }
    }

    /// Resends the request to update state if too much time has passed
    pub fn request_again(&self) -> bool {
        if let Some(nanos) = self.check_time_since_update_request() {
            nanos > 1000000000
        } else {
            false
        }
    }

    /// Checks if the chain is missing the genesis block
    pub fn check_missing_genesis(&self) -> Option<ComponentTypes> {
        if !self.components_received.contains(&ComponentTypes::Genesis) {
            return Some(ComponentTypes::Genesis);
        }

        None
    }

    /// Checks if the chain is missing the Child Block
    pub fn check_missing_child(&self) -> Option<ComponentTypes> {
        if !self.components_received.contains(&ComponentTypes::Child) {
            return Some(ComponentTypes::Child);
        }

        None
    }

    /// Checks if the chain is missing the Parent Block
    pub fn check_missing_parent(&self) -> Option<ComponentTypes> {
        if !self.components_received.contains(&ComponentTypes::Parent) {
            return Some(ComponentTypes::Parent);
        }

        None
    }

    /// Checks if the chain is missing the current ledger
    pub fn check_missing_ledger(&self) -> Option<ComponentTypes> {
        if !self.components_received.contains(&ComponentTypes::Ledger) {
            return Some(ComponentTypes::Ledger);
        }

        None
    }

    /// Checks if the chain is missing the current network state
    pub fn check_missing_state(&self) -> Option<ComponentTypes> {
        if !self
            .components_received
            .contains(&ComponentTypes::NetworkState)
        {
            return Some(ComponentTypes::NetworkState);
        }

        None
    }

    /// Creates vector of all components missing from the chain.
    pub fn check_missing_components(&self) -> Vec<ComponentTypes> {
        let mut missing = vec![];
        if let Some(component) = self.check_missing_genesis() {
            missing.push(component);
        }

        if let Some(component) = self.check_missing_child() {
            missing.push(component);
        }

        if let Some(component) = self.check_missing_parent() {
            missing.push(component);
        }

        if let Some(component) = self.check_missing_ledger() {
            missing.push(component);
        }

        if let Some(component) = self.check_missing_state() {
            missing.push(component);
        }

        missing
    }

    /// Serializes a chain into bytes
    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_string().as_bytes().to_vec()
    }

    /// Deserialize a slice of bytes into a blockchain
    pub fn from_bytes(data: &[u8]) -> Blockchain {
        serde_json::from_slice::<Blockchain>(data).unwrap()
    }

    /// Serializes a chain into a string
    // TODO: Consider changing the name to `serialize_to_string`
    #[allow(clippy::inherent_to_string_shadow_display)]
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    /// Deserializes a string slice into a chain
    pub fn from_string(data: &str) -> Blockchain {
        serde_json::from_str(data).unwrap()
    }

    /// Returns a vector of all the field names of a chain.
    pub fn get_field_names(&self) -> Vec<String> {
        vec![
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
        ]
    }
}

impl fmt::Display for Blockchain {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Blockchain")
    }
}

impl Error for Blockchain {}

impl GettableFields for Blockchain {
    fn get_field(&self, field: &str) -> Option<String> {
        match field {
            "genesis" => self.genesis.clone().map(|g| g.to_string()),
            "child" => self.child.clone().map(|c| c.to_string()),
            "parent" => self.parent.clone().map(|p| p.to_string()),
            "chain" => Some(serde_json::to_string(&self.chain).unwrap()),
            "chain_db" => Some(self.chain_db.clone()),
            "block_cache" => Some(serde_json::to_string(&self.block_cache).unwrap()),
            "future_blocks" => Some(serde_json::to_string(&self.future_blocks).unwrap()),
            "invalid" => Some(serde_json::to_string(&self.invalid).unwrap()),
            "updating_state" => Some(format!("{}", self.updating_state)),
            "state_update_cache" => Some(serde_json::to_string(&self.state_update_cache).unwrap()),
            _ => None,
        }
    }
}
