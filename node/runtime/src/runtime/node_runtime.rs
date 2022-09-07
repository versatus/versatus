use std::str::FromStr;
/// Everything on this crate is tentative and meant to be a stepping stone into
/// the finalized version soon.
use std::{
    borrow::BorrowMut,
    collections::{HashMap, HashSet},
    env::args,
    fs,
    io::{Read, Write},
    net::{AddrParseError, SocketAddr, SocketAddrV4, SocketAddrV6, UdpSocket},
    path::PathBuf,
    rc::Rc,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
    time::{Duration, Instant},
};

use blockchain::blockchain::Blockchain;
use claim::claim::Claim;
use clap::Parser;
use commands::command::{Command, ComponentTypes};
use events::events::{write_to_json, VrrbNetworkEvent};
use ledger::ledger::Ledger;
use messages::message_types::MessageType;
use miner::miner::Miner;
use network::{components::StateComponent, message};
use node::{
    command_handler::CommandHandler,
    core::NodeType,
    message_handler::MessageHandler,
    node::{Node, NodeAuth},
};
use primitives::StopSignal;
use public_ip;
use rand::{thread_rng, Rng};
use reward::reward::{Category, RewardState};
use ritelinked::LinkedHashMap;
use simplelog::{Config, LevelFilter, WriteLogger};
use state::state::{Components, NetworkState};
use storage::FileSystemStorage;
use strum_macros::EnumIter;
use telemetry::info;
use thiserror::Error;
use tokio::sync::{
    mpsc::{self, UnboundedReceiver, UnboundedSender},
    oneshot::{self, error::TryRecvError},
};
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

use super::{blockchain::BlockchainModule, swarm::SwarmConfig};
use crate::{
    result::{Result, RuntimeError},
    runtime::{miner::MiningModule, state::StateModule, swarm::SwarmModule},
    RuntimeModule,
    RuntimeModuleState,
};

pub const VALIDATOR_THRESHOLD: f64 = 0.60;

#[derive(Debug, Clone)]
pub struct RuntimeOpts {
    pub node_type: node::node::NodeType,
    pub data_dir: PathBuf,
    pub node_idx: primitives::NodeIdx,
}

/// Runtime is responsible for initializing the node, handling networking and
/// config management
#[derive(Debug)]
pub struct Runtime {
    control_rx: oneshot::Receiver<primitives::StopSignal>,
    running_status: RuntimeModuleState,
}

impl Runtime {
    pub fn new(ctrl_rx: oneshot::Receiver<StopSignal>) -> Self {
        Self {
            control_rx: ctrl_rx,
            running_status: RuntimeModuleState::Stopped,
        }
    }

    /// Main node setup and execution entrypoint, called only by applications
    /// that intend to run VRRB nodes
    #[telemetry::instrument]
    pub async fn start(&mut self, opts: RuntimeOpts) -> Result<()> {
        // TODO: import and initialize things at node core
        telemetry::debug!("parsing runtime configuration");

        self.running_status = RuntimeModuleState::Running;

        // TODO: figure out what raw mode is
        // enable_raw_mode().expect("can run in raw mode");

        // let mut rng = rand::thread_rng();
        // let event_file_suffix: u8 = rng.gen();
        // let stdout = std::io::stdout();

        // Data directory setup
        // ___________________________________________________________________________________________________

        self.setup_data_dir(opts.data_dir.clone());

        // Setup log file and db files
        //____________________________________________________________________________________________________

        self.setup_log_and_db_file(opts.data_dir.clone());

        // //____________________________________________________________________________________________________
        // // ___________________________________________________________________________________________________
        // // setup message and command sender/receiver channels for communication
        // betwen various threads
        //
        // TODO: replace tx/rx setup with a routing table-like mechanism
        let (to_blockchain_sender, mut to_blockchain_receiver) = mpsc::unbounded_channel();
        let (to_miner_sender, mut to_miner_receiver) = mpsc::unbounded_channel();
        let (to_message_sender, mut to_message_receiver) = mpsc::unbounded_channel();
        let (from_message_sender, mut from_message_receiver) = mpsc::unbounded_channel();
        let (to_gossip_sender, mut to_gossip_receiver) = mpsc::unbounded_channel();
        let (command_sender, command_receiver) = mpsc::unbounded_channel();
        let (to_swarm_sender, mut to_swarm_receiver) = mpsc::unbounded_channel();
        let (to_state_sender, mut to_state_receiver) = mpsc::unbounded_channel();
        // let (to_app_sender, mut to_app_receiver) = mpsc::unbounded_channel();

        let (to_gossip_tx, to_gossip_rx) = std::sync::mpsc::channel();
        // let (to_kad_tx, to_kad_rx) = channel();
        // let (incoming_ack_tx, incoming_ack_rx) = channel();
        let (to_app_tx, _to_app_rx) = channel::<GossipMessage>();

        let (to_transport_tx, to_transport_rx): (
            Sender<(SocketAddr, Message)>,
            Receiver<(SocketAddr, Message)>,
        ) = channel();

        // let (incoming_ack_tx, incoming_ack_rx): (Sender<AckMessage>,
        // Receiver<AckMessage>) = channel();
        //____________________________________________________________________________________________________

        self.setup_wallet(opts.data_dir);

        //____________________________________________________________________________________________________
        // Node initialization

        let to_message_handler = MessageHandler::new(from_message_sender, to_message_receiver);

        let command_handler = CommandHandler::new(
            to_miner_sender,
            to_blockchain_sender,
            to_gossip_sender,
            to_swarm_sender,
            to_state_sender,
            to_gossip_tx,
            command_receiver,
        );

        let mut node = Node::new(
            opts.node_type,
            command_handler,
            to_message_handler,
            opts.node_idx,
        );

        /*
        //____________________________________________________________________________________________________
        // Swarm initialization
        // Need to replace swarm with custom swarm-like struct.

        // TODO: figure out what to do with these older values
        let pub_ip = public_ip::addr_v4().await.unwrap();
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
            local_socket_addr: local_sock.clone(),
            pub_socket_addr: local_sock.clone(),
            udp_socket: sock,
        };

        let swarm = SwarmModule::new(swarm_config);
        swarm.start();

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
        let blockchain_module = BlockchainModule::new();
        blockchain_module.start();

        //____________________________________________________________________________________________________
        // Mining thread
        let minig_module = MiningModule::new();
        minig_module.start();

        //____________________________________________________________________________________________________
        // State Sending Thread
        let state_module = StateModule::new();
        state_module.start();



        //____________________________________________________________________________________________________

        */

        // let node_handle = tokio::spawn(async move {
        //     node.start().await?;
        // })
        // .await;

        // Runtime modules teardown
        //____________________________________________________________________________________________________

        loop {
            // TODO: rethink this loop

            match self.control_rx.try_recv() {
                Ok(sig) => {
                    telemetry::info!("Received stop signal");
                    self.teardown();
                    break;
                },
                Err(err) if err == TryRecvError::Closed => {
                    telemetry::warn!("Failed to process stop signal. Reason: {0}", err);
                    telemetry::warn!("Shutting down");
                    break;
                },
                _ => {},
            }
        }

        telemetry::info!("Node shutting down");

        Ok(())
    }

    fn setup_data_dir(&self, data_dir: PathBuf) -> Result<()> {
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
            telemetry::error!("{:?}", err.to_string());
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

    pub fn status(&self) -> RuntimeModuleState {
        self.running_status.clone()
    }

    fn teardown(&mut self) {
        self.running_status = RuntimeModuleState::Stopped;
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
//

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, rc::Rc, sync::Arc};

    use node::core::NodeType;
    use telemetry::TelemetrySubscriber;
    use tokio::sync::oneshot;

    use super::Runtime;
    use crate::{RuntimeModuleState, RuntimeOpts};

    #[tokio::test]
    async fn node_runtime_starts_and_stops() {
        let (ctrl_tx, ctrl_rx) = oneshot::channel();

        let rt_opts = RuntimeOpts {
            node_type: NodeType::Full,
            data_dir: PathBuf::from("/tmp/vrrb"),
            node_idx: 100,
        };

        let mut node_rt = Runtime::new(ctrl_rx);
        assert_eq!(node_rt.status(), RuntimeModuleState::Stopped);

        let handle = tokio::spawn(async move {
            node_rt.start(rt_opts).await.unwrap();
            assert_eq!(node_rt.status(), RuntimeModuleState::Stopped);
        });

        ctrl_tx.send(primitives::StopSignal).unwrap();

        handle.await.unwrap();
    }
}
