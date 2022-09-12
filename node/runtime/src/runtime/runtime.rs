use std::{
    collections::{HashMap, HashSet},
    env::args,
    fs,
    io::{Read, Write},
    net::{SocketAddr, SocketAddrV4, SocketAddrV6, UdpSocket},
    sync::mpsc::{channel, Receiver, Sender},
    time::{Duration, Instant},
};

use block::{block, invalid::InvalidBlockErrorReason};
use blockchain::blockchain::Blockchain;
use claim::claim::Claim;
use commands::command::{Command, ComponentTypes};
use events::events::{write_to_json, VrrbNetworkEvent};
use ledger::ledger::Ledger;
use messages::message_types::MessageType;
use miner::miner::Miner;
use network::{components::StateComponent, message};
use node::{
    handler::{CommandHandler, MessageHandler},
    node::{Node, NodeAuth},
};
use public_ip;
use rand::{thread_rng, Rng};
use reward::reward::{Category, RewardState};
use ritelinked::LinkedHashMap;
use simplelog::{Config, LevelFilter, WriteLogger};
use state::state::{Components, NetworkState};
use storage::FileSystemStorage;
use strum_macros::EnumIter;
use telemetry::info;
use tokio::sync::mpsc;
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
use unicode_width::UnicodeWidthStr;
use validator::validator::TxnValidator;
use wallet::wallet::WalletAccount;

pub const VALIDATOR_THRESHOLD: f64 = 0.60;

use std::str::FromStr;

/// Everything on this crate is tentative and meant to be a stepping stone into
/// the finalized version soon.
use clap::Parser;
use thiserror::Error;

use super::swarm::SwarmConfig;

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, RuntimeError>;

#[derive(Debug, Clone)]
pub struct RuntimeOpts {
    pub node_type: node::node::NodeType,
}

/// Runtime is responsible for initializing the node, handling networking and
/// config management
#[derive(Debug, Default)]
pub struct Runtime {
    //
}

impl Runtime {
    pub fn new() -> Self {
        Self::default()
    }

    /// Main node setup and execution entrypoint, called only by applications
    /// that intend to run VRRB nodes
    #[telemetry::instrument]
    pub async fn start(&self, opts: RuntimeOpts) -> Result<()> {
        // TODO: import and initialize things at node core
        telemetry::debug!("parsing runtime configuration");

        // TODO: figure out what raw mode is
        // enable_raw_mode().expect("can run in raw mode");

        let mut rng = rand::thread_rng();

        let stdout = std::io::stdout();
        let event_file_suffix: u8 = rng.gen();

        // Data directory setup
        // ___________________________________________________________________________________________________

        self.setup_data_dir();

        //____________________________________________________________________________________________________
        // Setup log file and db files

        self.setup_log_and_db_file();

        // //____________________________________________________________________________________________________
        // // ___________________________________________________________________________________________________
        // // setup message and command sender/receiver channels for communication
        // betwen various threads let (to_blockchain_sender, mut
        // to_blockchain_receiver) = mpsc::unbounded_channel();
        // let (to_miner_sender, mut to_miner_receiver) = mpsc::unbounded_channel();
        // let (to_message_sender, mut to_message_receiver) = mpsc::unbounded_channel();
        // let (from_message_sender, mut from_message_receiver) =
        // mpsc::unbounded_channel(); let (to_gossip_sender, mut
        // to_gossip_receiver) = mpsc::unbounded_channel(); let (command_sender,
        // command_receiver) = mpsc::unbounded_channel(); let (to_swarm_sender,
        // mut to_swarm_receiver) = mpsc::unbounded_channel();
        // let (to_state_sender, mut to_state_receiver) = mpsc::unbounded_channel();
        // let (to_app_sender, mut to_app_receiver) = mpsc::unbounded_channel();
        // let (to_transport_tx, to_transport_rx): (
        //     Sender<(SocketAddr, Message)>,
        //     Receiver<(SocketAddr, Message)>,
        // ) = channel();
        // let (to_gossip_tx, to_gossip_rx) = channel();
        // let (to_kad_tx, to_kad_rx) = channel();
        // let (incoming_ack_tx, incoming_ack_rx): (Sender<AckMessage>,
        // Receiver<AckMessage>) = channel(); let (to_app_tx, _to_app_rx) =
        // channel::<GossipMessage>();
        //____________________________________________________________________________________________________

        self.setup_wallet();

        //____________________________________________________________________________________________________
        // Node initialization
        // call node_setup()
        let node = self.setup_node();

        //____________________________________________________________________________________________________
        // Swarm initialization
        // Need to replace swarm with custom swarm-like struct.

        // TODO: figure out what to do with these older values
        let pub_ip = public_ip::addr_v4().await;
        let port: usize = 19292;
        // let port: usize = thread_rng().gen_range(9292..19292);

        let addr = format!("{:?}:{:?}", pub_ip, port.clone());
        let local_sock: SocketAddr = addr.parse()?;
        //     .expect(
        //     "unable to parse
        // // address",
        // );

        // Bind a UDP Socket to a Socket Address with a random port between
        // 9292 and 19292 on the localhost address.
        let sock = UdpSocket::bind(format!("0.0.0.0:{:?}", port.clone()))?;
        // .expect("Unable to bind to address");

        let swarm_config = SwarmConfig {
            port,
            ip_address: pub_ip,
            local_socket_addr: local_sock,
            pub_socket_addr: sock,
        };

        let swarm = crate::runtime::swarm::SwarmModule::new(swarm_config);

        // Inform the local node of their address (since the port is randomized)
        //
        //____________________________________________________________________________________________________
        // Dial peer if provided
        //
        //____________________________________________________________________________________________________
        // Swarm event thread
        // Clone the socket for the transport and message handling thread(s)
        //
        // TODO (Daniel):  call Swarm::start here
        //____________________________________________________________________________________________________
        // Node startup thread
        // call Node::start() here
        // tokio::task::spawn(async move {
        //     if let Err(_) = node.start().await {
        //         panic!("Unable to start node!")
        //     };
        // });
        //____________________________________________________________________________________________________
        // Blockchain thread setup
        //____________________________________________________________________________________________________
        // Mining thread
        //____________________________________________________________________________________________________
        // State Sending Thread
        //____________________________________________________________________________________________________

        telemetry::info!("node shutting down");

        Ok(())
    }

    // All methods defined below are temporary placeholders of the actual steps
    // meant to be run
    //
    fn setup_node(&self) -> Node {
        let to_message_handler =
            MessageHandler::new(from_message_sender.clone(), to_message_receiver);
        let command_handler = CommandHandler::new(
            to_miner_sender.clone(),
            to_blockchain_sender.clone(),
            to_gossip_sender.clone(),
            to_swarm_sender.clone(),
            to_state_sender.clone(),
            to_gossip_tx.clone(),
            command_receiver,
        );

        let mut node = Node::new(node_type.clone(), command_handler, to_message_handler, 100);
        let node_id = node.id.clone();
        let node_key = node.pubkey.clone();

        node
    }

    fn setup_data_dir(&self) {
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
            telemetry::error!("{:?}", err.to_string());
        }
    }

    fn setup_log_and_db_file(&self) {
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
    }

    fn setup_wallet(&self) {
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
    }
}

// this appears to be the keyboard events loop
// let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
// let tick_rate = tokio::time::Duration::from_millis(200);
//
// std::thread::spawn(move || {
//     let mut last_tick = Instant::now();
//     loop {
//         let timeout = tick_rate
//             .checked_sub(last_tick.elapsed())
//             .unwrap_or_else(|| Duration::from_secs(0));
//
//         if event::poll(timeout).expect("poll works") {
//             if let CEvent::Key(key) = event::read().expect("can read events")
// {                 if let Err(_) = tx.send(Event::Input(key)) {
//                     info!("Can't send events");
//                 }
//             }
//         }
//
//         if last_tick.elapsed() > tick_rate {
//             if let Ok(_) = tx.send(Event::Tick) {
//                 last_tick = Instant::now();
//             }
//         }
//     }
// });
