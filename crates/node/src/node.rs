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

use lr_trie::LeftRightTrie;
use patriecia::db::MemoryDB;
use primitives::types::{
    rand,
    NodeId,
    NodeIdentifier,
    NodeIdx,
    PublicKey,
    Secp256k1,
    SecretKey,
    StopSignal,
};
use public_ip;
use serde::{Deserialize, Serialize};
use state::NetworkState;
use telemetry::{error, info, Instrument};
use thiserror::Error;
use tokio::sync::mpsc::{self, error::TryRecvError, UnboundedReceiver, UnboundedSender};
use trecho::vm::Cpu;
use uuid::Uuid;
use vrrb_core::event_router::{DirectedEvent, Event, EventRouter, Topic};
use vrrb_rpc::http::{HttpApiServer, HttpApiServerConfig};

use crate::{
    miner::MiningModule,
    result::*,
    runtime::blockchain_module::BlockchainModule,
    swarm::{SwarmConfig, SwarmModule},
    NodeAuth,
    NodeType,
    RuntimeModule,
    RuntimeModuleState,
    StateModule,
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

    /// The address of the bootstrap node(s), used for peer discovery and
    /// initial state sync
    bootstrap_node_addresses: Vec<SocketAddr>,

    /// VRRB world state. it contains the accounts tree
    // state: LeftRightTrie<MemoryDB>,

    /// Confirmed transactions
    // txns: LeftRightTrie<MemoryDB>,

    /// Unconfirmed transactions
    // mempool: LeftRightTrie<MemoryDB>,

    // validator_unit: Option<i32>,
    running_status: RuntimeModuleState,

    vm: Cpu,

    http_api_server_config: HttpApiServerConfig,
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

        let http_api_server_config = HttpApiServerConfig {
            address: config.http_api_address.to_string(),
            api_title: config.http_api_title.clone(),
            api_version: config.http_api_version.clone(),
            server_timeout: config.http_api_shutdown_timeout.clone(),
        };

        let bootstrap_node_addresses = config.bootstrap_node_addresses.clone();

        Self {
            id: config.id.clone(),
            idx: config.idx.clone(),
            node_type: config.node_type.clone(),
            secret_key,
            pubkey: pubkey.to_string(),
            public_key: pubkey,
            is_bootsrap: config.bootstrap,
            bootstrap_node_addresses,
            running_status: RuntimeModuleState::Stopped,
            data_dir: config.data_dir().clone(),
            vm,
            http_api_server_config,
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

    pub fn status(&self) -> RuntimeModuleState {
        self.running_status.clone()
    }

    fn set_status(&mut self, status: RuntimeModuleState) {
        self.running_status = status;
    }

    fn teardown(&mut self) {
        self.running_status = RuntimeModuleState::Stopped;
    }

    /// Main node setup and execution entrypoint, called only by applications
    /// that intend to run VRRB nodes
    #[telemetry::instrument]
    pub async fn start(&mut self, control_rx: &mut UnboundedReceiver<Event>) -> Result<()> {
        telemetry::debug!("parsing runtime configuration");

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

        let (http_server_control_tx, mut http_server_control_rx) =
            tokio::sync::mpsc::channel::<()>(1);

        let http_api_server =
            HttpApiServer::new(self.http_api_server_config.clone()).map_err(|err| {
                NodeError::Other(format!("Unable to create API server. Reason: {}", err))
            })?;

        let http_server_handle = tokio::spawn(async move {
            http_api_server.start(&mut http_server_control_rx).await;
        });

        self.set_status(RuntimeModuleState::Running);

        // Runtime module teardown
        //____________________________________________________________________________________________________
        // TODO: start node API here
        loop {
            match control_rx.try_recv() {
                Ok(evt) => {
                    telemetry::info!("Received stop event");

                    http_server_control_tx.send(());

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

    use vrrb_config::NodeConfig;

    use super::*;
    use crate::test_utils::create_mock_full_node_config;

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
