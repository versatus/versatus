use std::{
    borrow::BorrowMut,
    collections::{HashMap, HashSet},
    env::args,
    fs,
    io::{Read, Write},
    net::{AddrParseError, SocketAddr, SocketAddrV4, SocketAddrV6, UdpSocket},
    path::PathBuf,
    rc::Rc,
    str::FromStr,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
    time::{Duration, Instant},
};

use trecho::vm::Cpu;
use vrrb_core::event_router::{DirectedEvent, Event, EventRouter, Topic};

use block::Block;
use claim::claim::Claim;
use commands::command::{Command, ComponentTypes};
use events::events::{write_to_json, VrrbNetworkEvent};
use ledger::ledger::Ledger;
use lr_trie::LeftRightTrie;
use messages::{
    message_types::MessageType,
    packet::{Packet, Packetize},
};
use miner::miner::Miner;
use network::{components::StateComponent, message};
use patriecia::db::MemoryDB;
use pickledb::PickleDb;
use poem::listener::Listener;
use primitives::types::{NodeId, NodeIdentifier, NodeIdx, PublicKey, SecretKey, StopSignal};
use public_ip;
use rand::{thread_rng, Rng};
use reward::reward::{Category, RewardState};
use ritelinked::LinkedHashMap;
use secp256k1::Secp256k1;
use serde::{Deserialize, Serialize};
use state::{Components, NetworkState};
use telemetry::{error, info, Instrument};
use thiserror::Error;
use tokio::sync::mpsc::{self, error::TryRecvError, UnboundedReceiver, UnboundedSender};
use txn::txn::Txn;
use udp2p::{
    discovery::{kad::Kademlia, routing::RoutingTable},
    gossip::{
        gossip::{GossipConfig, GossipService},
        protocol::GossipMessage,
    },
    node::{peer_id::PeerId, peer_info::PeerInfo, peer_key::Key},
    protocol::protocol::{packetize, AckMessage, Header, Message, MessageKey},
    transport::{handler::MessageHandler as GossipMessageHandler, transport::Transport},
    utils::utils::ByteRep,
};
use uuid::Uuid;
use wallet::wallet::WalletAccount;

use crate::{
    miner::MiningModule,
    result::*,
    runtime::blockchain_module::BlockchainModule,
    swarm::{SwarmConfig, SwarmModule},
    NodeAuth, NodeType, RuntimeModule, RuntimeModuleState, StateModule,
};

pub const VALIDATOR_THRESHOLD: f64 = 0.60;

/// Node represents a member of the VRRB network and it is responsible for
/// carrying out the different operations permitted within the chain.
#[derive(Debug)]
pub struct Node {
    /// Every node needs a unique ID to identify it as a member of the network.
    pub id: NodeIdentifier,

    /// Index of the node in the network
    pub idx: NodeIdx,

    /// Every node needs to have a secret key to sign messages, blocks, tx, etc.
    /// for authenticity
    //TODO: Discuss whether we need this here or whether it's redundant.
    pub secret_key: SecretKey,

    /// Every node needs to have a public key to have its messages, blocks, tx,
    /// etc, signatures validated by other nodes
    //TODOL: Discuss whether this is needed here.
    pub pubkey: String,
    pub public_key: PublicKey,

    /// The type of the node, used for custom impl's based on the type the
    /// capabilities may vary.
    //TODO: Change this to a generic that takes anything that implements the NodeAuth trait.
    //TODO: Create different custom structs for different kinds of nodes with different
    // authorization so that we can have custom impl blocks based on the type.
    pub node_type: NodeType,

    /// Directory used to persist all VRRB node information to disk
    data_dir: PathBuf,

    /// Whether the current node is a bootstrap node or not
    is_bootsrap: bool,

    /// The address of the bootstrap node, used for peer discovery and initial state sync
    bootsrap_addr: SocketAddr,

    /// VRRB world state. it contains the accounts tree
    // state: LeftRightTrie<MemoryDB>,

    /// Confirmed transactions
    // txns: LeftRightTrie<MemoryDB>,

    /// Unconfirmed transactions
    // mempool: LeftRightTrie<MemoryDB>,

    // validator_unit: Option<i32>,
    running_status: RuntimeModuleState,

    vm: Cpu,
}

impl Node {
    /// Creates and returns a Node instance
    pub fn new(config: vrrb_config::NodeConfig) -> Node {
        let secp = Secp256k1::new();
        let mut rng = rand::thread_rng();
        let (secret_key, pubkey) = secp.generate_keypair(&mut rng);
        let vm = trecho::vm::Cpu::new();

        //TODO: use SecretKey from threshold crypto crate for MasterNode
        //TODO: Discussion :Generation/Serializing/Deserialzing of secret key to be
        // moved to primitive/utils module
        let mut secret_key_encoded = Vec::new();

        Self {
            id: config.id.clone(),
            idx: config.idx.clone(),
            node_type: config.node_type.clone(),
            secret_key: secret_key_encoded,
            pubkey: pubkey.to_string(),
            public_key: pubkey.to_string().into_bytes(),
            is_bootsrap: config.bootstrap,
            bootsrap_addr: config.bootstrap_node_addr,
            running_status: RuntimeModuleState::Stopped,
            data_dir: config.data_dir().clone(),
            vm,
        }
    }

    /// Returns a string representation of the node id
    pub fn get_id(&self) -> String {
        self.id.clone()
    }

    /// Returns the type of the node
    pub fn get_node_type(&self) -> NodeType {
        self.node_type.clone()
    }

    /// Returns the idx of the node
    pub fn get_node_idx(&self) -> u16 {
        self.idx
    }

    pub fn is_bootsrap(&self) -> bool {
        self.is_bootsrap
    }

    pub fn bootsrap_addr(&self) -> SocketAddr {
        self.bootsrap_addr
    }

    pub fn status(&self) -> RuntimeModuleState {
        self.running_status.clone()
    }

    fn teardown(&mut self) {
        self.running_status = RuntimeModuleState::Stopped;
    }

    /// Main node setup and execution entrypoint, called only by applications
    /// that intend to run VRRB nodes
    #[telemetry::instrument]
    pub async fn start(&mut self, control_rx: &mut UnboundedReceiver<Event>) -> Result<()> {
        telemetry::debug!("parsing runtime configuration");

        self.running_status = RuntimeModuleState::Running;

        // TODO: replace memorydb with real backing db later
        let mem_db = MemoryDB::new(true);
        let backing_db = Arc::new(mem_db);
        let lr_trie = LeftRightTrie::new(backing_db);

        // TODO: setup other modules

        //____________________________________________________________________________________________________
        // State module
        let (state_control_tx, mut state_control_rx) =
            tokio::sync::mpsc::unbounded_channel::<Event>();

        let mut state_module = StateModule::new("".into());

        let state_handle = tokio::spawn(async move {
            state_module.start(&mut state_control_rx);
        });

        let (router_control_tx, mut router_control_rx) =
            tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();

        let mut event_router = EventRouter::new();

        // event_router.add_subscriber(Topic::Control, state_control_tx.clone());

        // TODO: report error from handle
        let router_handle = tokio::spawn(async move {
            // TODO: fix blocking loop on router
            event_router.start(&mut router_control_rx).await
            // }
        });

        // Runtime module teardown
        //____________________________________________________________________________________________________
        // TODO: start node API here
        loop {
            match control_rx.try_recv() {
                Ok(evt) => {
                    telemetry::info!("Received stop event");

                    // TODO: send signal to stop all task handlers here
                    router_control_tx
                        .send((Topic::Control, evt))
                        .unwrap_or_default();

                    self.teardown();

                    break;
                },
                Err(err) if err == TryRecvError::Disconnected => {
                    telemetry::warn!("Failed to process stop signal. Reason: {0}", err);
                    telemetry::warn!("Shutting down");
                    break;
                },
                _ => {},
            }
        }
        //
        // TODO: await on all task handles here

        telemetry::info!("Node shutdown complete");

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use std::{
        env,
        net::{IpAddr, Ipv4Addr},
    };

    use super::*;
    use crate::test_utils::create_mock_full_node_config;
    use vrrb_config::NodeConfig;

    #[test]
    fn node_teardown_updates_node_status() {
        let node_config = create_mock_full_node_config();

        let mut vrrb_node = Node::new(node_config);
        assert_eq!(vrrb_node.status(), RuntimeModuleState::Stopped);

        vrrb_node.running_status = RuntimeModuleState::Running;
        assert_eq!(vrrb_node.status(), RuntimeModuleState::Running);

        vrrb_node.teardown();
        assert_eq!(vrrb_node.status(), RuntimeModuleState::Stopped);
    }
}
