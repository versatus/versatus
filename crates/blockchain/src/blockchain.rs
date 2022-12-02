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
use messages::message_types::MessageType;
use pickledb::{PickleDb, PickleDbDumpPolicy, SerializationMethod};
use reward::reward::Reward;
use ritelinked::LinkedHashMap;
use serde::{Deserialize, Serialize};
use state::state::NetworkState;
use telemetry::info;
use udp2p::{
    gossip::protocol::GossipMessage,
    protocol::protocol::{Header, Message, MessageKey},
    utils::utils::{timestamp_now, ByteRep},
};
use vrrb_core::verifiable::Verifiable;
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

    // ========================================================================================================
    //TODO: ChainDb ops
    // ========================================================================================================

    // ========================================================================================================
    //TODO: BlockProcessor ops
    // ========================================================================================================

    // ========================================================================================================
    //TODO: SyncReporter ops
    // ========================================================================================================

    // ========================================================================================================
    //TODO: IntegrityChecker ops
    // ========================================================================================================
}

impl fmt::Display for Blockchain {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Blockchain")
    }
}
