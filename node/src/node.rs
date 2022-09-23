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
use primitives::{NodeId, NodeIdx, StopSignal};
use public_ip;
use rand::{thread_rng, Rng};
use reward::reward::{Category, RewardState};
use ritelinked::LinkedHashMap;
use secp256k1::Secp256k1;
use serde::{Deserialize, Serialize};
use state::state::{Components, NetworkState};
use storage::FileSystemStorage;
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
    command_handler::CommandHandler,
    command_router::{self, CommandRoute, DirectedCommand},
    message_handler::MessageHandler,
    miner::MiningModule,
    result::*,
    runtime::blockchain::BlockchainModule,
    state::StateModule,
    swarm::{SwarmConfig, SwarmModule},
    NodeAuth, NodeType, RuntimeModule, RuntimeModuleState,
};

pub const VALIDATOR_THRESHOLD: f64 = 0.60;

/// Node represents a member of the VRRB network and it is responsible for
/// carrying out the different operations permitted within the chain.
#[derive(Debug)]
pub struct Node {
    /// Every node needs a unique ID to identify it as a member of the network.
    pub id: primitives::NodeIdentifier,

    /// Index of the node in the network
    pub idx: NodeIdx,

    /// Every node needs to have a secret key to sign messages, blocks, tx, etc.
    /// for authenticity
    //TODO: Discuss whether we need this here or whether it's redundant.
    pub secret_key: primitives::SecretKey,

    /// Every node needs to have a public key to have its messages, blocks, tx,
    /// etc, signatures validated by other nodes
    //TODOL: Discuss whether this is needed here.
    pub pubkey: String,
    pub public_key: primitives::PublicKey,

    /// The type of the node, used for custom impl's based on the type the
    /// capabilities may vary.
    //TODO: Change this to a generic that takes anything that implements the NodeAuth trait.
    //TODO: Create different custom structs for different kinds of nodes with different
    // authorization so that we can have custom impl blocks based on the type.
    pub node_type: primitives::NodeType,

    /// The command handler used to allocate commands to different parts of the
    /// system
    // pub command_handler: CommandHandler,

    /// The message handler used to convert received messages into a command and
    /// to structure and pack outgoing messages to be sent to the transport
    /// layer
    // pub message_handler: MessageHandler<MessageType, (Packet, SocketAddr)>,
    data_dir: PathBuf,
    // control_rx: UnboundedReceiver<Command>,
    is_bootsrap: bool,
    bootsrap_addr: SocketAddr,
    // db_path: PathBuf,
    // state: LeftRightTrie<MemoryDB>,
    // txns: LeftRightTrie<MemoryDB>,
    // mempool: HashMap<String, String>,
    // validator_unit: Option<i32>,
    running_status: RuntimeModuleState,
}

impl Node {
    /// Creates and returns a Node instance
    pub fn new(config: vrrb_config::NodeConfig) -> Node {
        let secp = Secp256k1::new();
        let mut rng = rand::thread_rng();
        let (secret_key, pubkey) = secp.generate_keypair(&mut rng);
        let id = Uuid::new_v4().to_simple().to_string();

        //TODO: use SecretKey from threshold crypto crate for MasterNode
        //TODO: Discussion :Generation/Serializing/Deserialzing of secret key to be
        // moved to primitive/utils module
        let mut secret_key_encoded = Vec::new();

        /*
        let new_secret_wrapped =SerdeSecret(secret_key);
        let mut secret_key_encoded = Vec::new();
        if node_type==NodeType::MasterNode{
            secret_key_encoded=bincode::serialize(&new_secret_wrapped).unwrap();
        }
        */

        Self {
            id: config.id.clone(),
            idx: config.idx.clone(),
            node_type: config.node_type.clone(),
            secret_key: secret_key_encoded,
            pubkey: pubkey.to_string(),
            public_key: pubkey.to_string().into_bytes(),
            // db_path: todo!(),
            // state: todo!(),
            // txns: todo!(),
            // mempool: todo!(),
            // validator_unit: todo!(),
            is_bootsrap: config.bootstrap,
            bootsrap_addr: config.bootstrap_node_addr,
            running_status: RuntimeModuleState::Stopped,
            data_dir: config.data_dir().clone(),
            // control_rx: todo!(),
            // command_handler: todo!(),
            // message_handler: todo!(),
            //
            //
            // TODO: refactor these values
            // public_key: pubkey.to_string().to_vec(),
            // message_cache: HashSet::new(),
            // packet_storage: HashMap::new(),
            // command_handler,
            // message_handler,
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
    pub async fn start(&mut self, control_rx: &mut UnboundedReceiver<Command>) -> Result<()> {
        telemetry::debug!("parsing runtime configuration");

        let (router_control_tx, mut router_control_rx) =
            tokio::sync::mpsc::unbounded_channel::<DirectedCommand>();

        let mut cmd_router = command_router::CommandRouter::new();

        self.running_status = RuntimeModuleState::Running;
        // TODO: publish that node is running

        // TODO: replace memorydb with real backing db later
        let mem_db = MemoryDB::new(true);
        let backing_db = Arc::new(mem_db);
        let lr_trie = LeftRightTrie::new(backing_db);

        // Data directory setup
        // TODO: setup storage facade crate
        // ___________________________________________________________________________________________________

        self.setup_data_dir(self.data_dir.clone());
        self.setup_log_and_db_file(self.data_dir.clone());
        self.setup_wallet(self.data_dir.clone());

        //____________________________________________________________________________________________________
        // Swarm module
        // Need to replace swarm with custom swarm-like struct.

        // TODO: join all handles and route commands through router
        // TODO: figure out what to do with these older values

        let pub_ip = public_ip::addr_v4().await.unwrap();
        let port: usize = 19292;
        // let port: usize = thread_rng().gen_range(9292..19292);

        let addr = format!("{:?}:{:?}", pub_ip, port.clone());
        let local_sock: SocketAddr = addr.parse()?;

        // Bind a UDP Socket to a Socket Address with a random port between
        // 9292 and 19292 on the localhost address.
        let sock = UdpSocket::bind(format!("0.0.0.0:{:?}", port.clone()))?;
        // .expect("Unable to bind to address");

        let swarm_config = SwarmConfig {
            port,
            ip_address: pub_ip,
            local_socket_addr: local_sock.clone(),
            pub_socket_addr: local_sock.clone(),
            udp_socket: sock,
        };

        let (swarm_control_tx, mut swarm_control_rx) =
            tokio::sync::mpsc::unbounded_channel::<Command>();

        let mut swarm = SwarmModule::new(swarm_config);

        let swarm_handle = tokio::spawn(async move {
            swarm.start(&mut swarm_control_rx);
        });

        let (blockchain_control_tx, mut blockchain_control_rx) =
            tokio::sync::mpsc::unbounded_channel::<Command>();

        let mut blockchain_module = BlockchainModule::new(router_control_tx.clone());

        let blockchain_handle = tokio::spawn(async move {
            blockchain_module.start(&mut blockchain_control_rx);
        });

        //____________________________________________________________________________________________________
        // Mining module

        let (mining_control_tx, mut mining_control_rx) =
            tokio::sync::mpsc::unbounded_channel::<Command>();

        let mut minig_module = MiningModule::new();

        let mining_handle = tokio::spawn(async move {
            minig_module.start(&mut mining_control_rx);
        });

        //____________________________________________________________________________________________________
        // State module

        let (state_control_tx, mut state_control_rx) =
            tokio::sync::mpsc::unbounded_channel::<Command>();

        let mut state_module = StateModule::new();

        let state_handle = tokio::spawn(async move {
            state_module.start(&mut state_control_rx);
        });

        // NOTE: setup the command subscribers
        cmd_router.add_subscriber(CommandRoute::Blockchain, blockchain_control_tx.clone())?;
        cmd_router.add_subscriber(CommandRoute::Miner, mining_control_tx.clone())?;
        cmd_router.add_subscriber(CommandRoute::Swarm, swarm_control_tx.clone())?;
        cmd_router.add_subscriber(CommandRoute::State, state_control_tx.clone())?;

        // TODO: feed command handler to transport layer
        // TODO: report error from handle
        let router_handle = tokio::spawn(async move {
            // TODO: fix blocking loop on router
            // if let Err(err) = cmd_router.start(&mut router_control_rx).await {
            //     telemetry::error!("error while listening for commands: {0}", err);
            // }
        });

        // Runtime module teardown
        //____________________________________________________________________________________________________
        // TODO: start node API here
        // TODO: rethink this loop
        loop {
            match control_rx.try_recv() {
                Ok(sig) => {
                    telemetry::info!("Received stop signal");

                    // TODO: send signal to stop all task handlers here
                    router_control_tx
                        .send((CommandRoute::Router, Command::Stop))
                        .unwrap();

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

        // TODO: await on all task handles here

        telemetry::info!("Node shutdown complete");

        Ok(())
    }

    fn setup_data_dir(&self, data_dir: PathBuf) -> Result<()> {
        // TODO: decide who to feed this data dir
        let data_dir = storage::create_node_data_dir()?;

        /*
        let data_dir = String::from(".vrrb").into();
        let fs_storage = FileSystemStorage::new(data_dir);

        let directory = {
            if let Some(dir) = std::env::args().nth(2) {
                std::fs::create_dir_all(dir.clone())?;
                dir.clone()
            } else {
                std::fs::create_dir_all("./.vrrb_data".to_string())?;
                "./.vrrb_data".to_string()
            }
        };

        let events_path = format!("{}/events_{}.json", directory.clone(), event_file_suffix);
        fs::File::create(events_path.clone()).unwrap();
        if let Err(err) = write_to_json(events_path.clone(), VrrbNetworkEvent::VrrbStarted) {
            info!("Error writting to json in main.rs 164");
            error!("{:?}", err.to_string());
        }
        */

        Ok(())
    }

    fn setup_log_and_db_file(&self, data_dir: PathBuf) -> Result<()> {
        /*
        let node_type = NodeAuth::Full;
        let log_file_suffix: u8 = rng.gen();
        let log_file_path = if let Some(path) = std::env::args().nth(4) {
            path
        } else {
            format!(
                "{}/vrrb_log_file_{}.log",
                directory.clone(),
                log_file_suffix
            )
        };
        let _ = WriteLogger::init(
            LevelFilter::Info,
            Config::default(),
            fs::File::create(log_file_path).unwrap(),
        );

        */
        Ok(())
    }

    fn setup_wallet(&self, data_dir: PathBuf) -> Result<()> {
        /*
        let wallet = if let Some(secret_key) = std::env::args().nth(3) {
            WalletAccount::restore_from_private_key(secret_key)
        } else {
            WalletAccount::new()
        };

        let mut rng = rand::thread_rng();
        let file_suffix: u32 = rng.gen();
        let path = if let Some(path) = std::env::args().nth(5) {
            path
        } else {
            format!("{}/test_{}.json", directory.clone(), file_suffix)
        };

        let mut network_state = NetworkState::restore(&path);
        let ledger = Ledger::new();
        network_state.set_ledger(ledger.as_bytes());
        let reward_state = RewardState::start();
        network_state.set_reward_state(reward_state);
        */

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

    #[test]
    #[ignore = "not implemented yet"]
    fn should_create_node_from_valid_config() {
        todo!();
    }
}
