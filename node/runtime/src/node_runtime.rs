use std::{
    collections::{HashMap, HashSet},
    env::args,
    fs,
    io::{Read, Write},
    net::{SocketAddr, SocketAddrV4, SocketAddrV6, UdpSocket},
    sync::mpsc::{channel, Receiver, Sender},
    time::{Duration, Instant},
};

use ::block::invalid::InvalidBlockErrorReason;
use block::block;
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
use wallet::wallet::WalletAccount;

pub const VALIDATOR_THRESHOLD: f64 = 0.60;

use std::str::FromStr;

/// Everything on this crate is tentative and meant to be a stepping stone into
/// the finalized version soon.
use clap::Parser;
use thiserror::Error;

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
pub struct Runtime {}

impl Runtime {
    pub fn new() -> Self {
        Self::default()
    }

    #[telemetry::instrument]
    /// Main node setup and execution entrypoint, called only by applications
    /// that intend to run VRRB nodes
    // TODO replace anyhow::Result with custom result using RuntimeError instead
    pub async fn start(&self, opts: RuntimeOpts) -> Result<()> {
        //
        // TODO: import and initialize things at node core
        telemetry::debug!("parsing runtime configuration");

        // let node_runtime = Runtime::new();
        // node_runtime.start().await?;

        // TODO: figure out what raw mode is
        // enable_raw_mode().expect("can run in raw mode");

        let mut rng = rand::thread_rng();

        //
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

        let stdout = std::io::stdout();
        let event_file_suffix: u8 = rng.gen();

        // Data directory setup
        // ___________________________________________________________________________________________________
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

        //____________________________________________________________________________________________________
        // Setup log file and db files

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

        //____________________________________________________________________________________________________
        // Node initialization
        // call node_setup()
        //
        //____________________________________________________________________________________________________
        //
        //____________________________________________________________________________________________________
        // Swarm initialization
        // Need to replace swarm with custom swarm-like struct.
        //
        // // Inform the local node of their address (since the port is randomized)
        // //____________________________________________________________________________________________________
        //
        // //____________________________________________________________________________________________________
        // // Dial peer if provided
        //
        // //____________________________________________________________________________________________________
        //
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
}

/*
fn setup_blockchain() {

    //____________________________________________________________________________________________________
    // Blockchain thread
    let mut blockchain_network_state = network_state.clone();
    let mut blockchain_reward_state = reward_state.clone();
    let blockchain_to_miner_sender = to_miner_sender.clone();
    let blockchain_to_swarm_sender = to_swarm_sender.clone();
    let blockchain_to_gossip_sender = to_gossip_tx.clone();
    let blockchain_to_blockchain_sender = to_blockchain_sender.clone();
    let blockchain_to_state_sender = to_state_sender.clone();
    let blockchain_to_app_sender = to_app_sender.clone();
    let blockchain_node_id = node_id.clone();
    std::thread::spawn(move || {
        let mut rng = rand::thread_rng();
        let file_suffix: u32 = rng.gen();
        let mut blockchain =
            Blockchain::new(&format!("{}/test_chain_{}.db", directory, file_suffix));
        if let Err(_) = blockchain_to_app_sender
            .send(Command::UpdateAppBlockchain(blockchain.clone().as_bytes()))
        {
            info!("Error sending blockchain update to App receiver.")
        }
        loop {
            let miner_sender = blockchain_to_miner_sender.clone();
            let swarm_sender = blockchain_to_swarm_sender.clone();
            let gossip_sender = blockchain_to_gossip_sender.clone();
            let state_sender = blockchain_to_state_sender.clone();
            let blockchain_sender = blockchain_to_blockchain_sender.clone();
            let app_sender = blockchain_to_app_sender.clone();
            // let blockchain_sender = blockchain_to_blockchain_sender.clone();
            if let Ok(command) = to_blockchain_receiver.try_recv() {
                match command {
                    Command::PendingBlock(block_bytes, sender_id) => {
                        let block = block::Block::from_bytes(&block_bytes);
                        if blockchain.updating_state {
                            blockchain
                                .future_blocks
                                .insert(block.clone().header.last_hash, block.clone());

                            if let Err(e) = app_sender
                                .send(Command::UpdateAppBlockchain(blockchain.clone().as_bytes()))
                            {
                                info!("Error sending Blockchain to app: {:?}", e);
                            }
                        } else {
                            if let Err(e) = blockchain.process_block(
                                &blockchain_network_state,
                                &blockchain_reward_state,
                                &block,
                            ) {
                                // TODO: Replace with Command::InvalidBlock being sent to the node or gossip
                                // and being processed.
                                // If the block is invalid because of BlockOutOfSequence Error request the missing blocks
                                // Or the current state of the network (first, missing blocks later)
                                // If the block is invalid because of a NotTallestChain Error tell the miner they are missing blocks.
                                // The miner should request the current state of the network and then all the blocks they are missing.
                                match e.details {
                                    InvalidBlockErrorReason::BlockOutOfSequence => {
                                        // Stash block in blockchain.future_blocks
                                        // Request state update once. Set "updating_state" field
                                        // in blockchain to true, so that it doesn't request it on
                                        // receipt of new future blocks which will also be invalid.
                                        blockchain.future_blocks.insert(block.header.last_hash.clone(), block.clone());
                                        if !blockchain.updating_state && !blockchain.processing_backlog {
                                            // send state request and set blockchain.updating state to true;
                                            info!("Error: {:?}", e);
                                            if let Some((_, v)) = blockchain.future_blocks.front() {
                                                let component = StateComponent::All;
                                                let message = MessageType::GetNetworkStateMessage {
                                                    sender_id: blockchain_node_id.clone(),
                                                    requested_from: sender_id.clone(),
                                                    requestor_address: addr.clone(),
                                                    requestor_node_type: node_type
                                                        .clone()
                                                        .as_bytes(),
                                                    lowest_block: v.header.block_height,
                                                    component: component.as_bytes(),
                                                };

                                                let msg_id = MessageKey::rand();
                                                let head = Header::Gossip;
                                                let gossip_msg = GossipMessage {
                                                    id: msg_id.inner(),
                                                    data: message.as_bytes(),
                                                    sender: addr.clone()
                                                };

                                                let msg = Message {
                                                    head,
                                                    msg: gossip_msg.as_bytes().unwrap()
                                                };

                                                let cloned_node_id = blockchain_node_id.clone();
                                                let thread_blockchain_sender = blockchain_sender.clone();
                                                std::thread::spawn(move || {

                                                    let thread_node_id = cloned_node_id.clone();
                                                    let listener = std::net::TcpListener::bind("0.0.0.0:19291").unwrap();
                                                    info!("Opened TCP listener for state update");
                                                    for stream in listener.incoming() {
                                                        let loop_blockchain_sender = thread_blockchain_sender.clone();
                                                        match stream {
                                                            Ok(mut stream) => {
                                                                info!("New connection: {}", stream.peer_addr().unwrap());
                                                                    let inner_node_id = thread_node_id.clone();
                                                                    std::thread::spawn(move || {
                                                                        let stream_blockchain_sender = loop_blockchain_sender.clone();
                                                                        let mut buf = [0u8; 655360];
                                                                        let mut bytes = vec![];
                                                                        let mut total = 0;
                                                                        'reader: loop {
                                                                            let res = stream.read(&mut buf);
                                                                            if let Ok(size) = res {
                                                                                total += size;
                                                                                buf[0..size].iter().for_each(|byte| {
                                                                                    bytes.push(*byte);
                                                                                });
                                                                                info!("Received total of {:?} bytes", total);
                                                                                if size == 0 {
                                                                                    info!("Received all bytes, reconstructing");
                                                                                    if let Some(message) = Message::from_bytes(&bytes) {
                                                                                        if let Some(gossip_msg) = GossipMessage::from_bytes(&message.msg) {
                                                                                            if let Some(message_type) = MessageType::from_bytes(&gossip_msg.data) {
                                                                                                info!("{:?}", message_type);
                                                                                                if let Some(command) = message::process_message(message_type, inner_node_id.clone(), addr.clone().to_string()) {
                                                                                                    if let Err(e) = stream_blockchain_sender.send(command) {
                                                                                                        info!("Error sending command to blockchain");
                                                                                                    }
                                                                                                }
                                                                                            }
                                                                                        };
                                                                                    }
                                                                                    break 'reader;
                                                                                }
                                                                            }
                                                                        }
                                                                        stream.shutdown(std::net::Shutdown::Both).expect("Unable to shutdown");
                                                                });
                                                            }
                                                            Err(e) => {}
                                                        }
                                                    }
                                                });

                                                info!("Requesting state update");
                                                if let Err(e) = gossip_sender
                                                    .send((addr.clone(), msg))
                                                {
                                                    info!("Error sending state update request to swarm sender: {:?}", e);
                                                };

                                                blockchain.updating_state = true;
                                                blockchain.started_updating = Some(udp2p::utils::utils::timestamp_now());
                                            }
                                        }
                                    }
                                    InvalidBlockErrorReason::NotTallestChain => {
                                        // Inform the miner they are missing blocks
                                        // info!("Error: {:?}", e);

                                    }
                                    _ => {
                                        if !blockchain.updating_state {
                                            let lowest_block = {
                                                if let Some(block) = blockchain.child.clone() {
                                                    block.clone()
                                                } else {
                                                    blockchain.genesis.clone().unwrap()
                                                }
                                            };
                                            info!("Error: {:?}", e);
                                            if block.header.block_height
                                                > lowest_block.header.block_height + 1
                                            {
                                                let component = StateComponent::All;
                                                let message = MessageType::GetNetworkStateMessage {
                                                    sender_id: blockchain_node_id.clone(),
                                                    requested_from: sender_id,
                                                    requestor_address: addr.clone(),
                                                    requestor_node_type: node_type
                                                        .clone()
                                                        .as_bytes(),
                                                    lowest_block: lowest_block.header.block_height,
                                                    component: component.as_bytes(),
                                                };

                                                let head = Header::Gossip;
                                                let msg_id = MessageKey::rand();
                                                let gossip_msg = GossipMessage {
                                                    id: msg_id.inner(),
                                                    data: message.as_bytes(),
                                                    sender: addr.clone()
                                                };
                                                let msg = Message {
                                                    head,
                                                    msg: gossip_msg.as_bytes().unwrap()
                                                };

                                                // TODO: Replace the below with sending to the correct channel
                                                if let Err(e) = gossip_sender
                                                    .send((addr.clone(), msg))
                                                {
                                                    info!("Error sending state update request to swarm sender: {:?}", e);
                                                };

                                                blockchain.updating_state = true;
                                            } else {
                                                // Miner is out of consensus tell them to update their state.
                                                let message = MessageType::InvalidBlockMessage {
                                                    block_height: block.header.block_height,
                                                    reason: e.details.as_bytes(),
                                                    miner_id: sender_id,
                                                    sender_id: blockchain_node_id.clone(),
                                                };

                                                let head = Header::Gossip;
                                                let msg_id = MessageKey::rand();
                                                let gossip_msg = GossipMessage {
                                                    id: msg_id.inner(),
                                                    data: message.as_bytes(),
                                                    sender: addr.clone()
                                                };
                                                let msg = Message {
                                                    head,
                                                    msg: gossip_msg.as_bytes().unwrap()
                                                };

                                                // TODO: Replace the below with sending to the correct channel
                                                if let Err(e) = gossip_sender
                                                    .send((addr.clone(), msg))
                                                {
                                                    info!("Error sending state update request to swarm sender: {:?}", e);
                                                };

                                                blockchain
                                                    .invalid
                                                    .insert(block.hash.clone(), block.clone());
                                            }
                                        }
                                    }
                                }

                                if let Err(_) = miner_sender
                                    .send(Command::InvalidBlock(block.clone().as_bytes()))
                                {
                                    info!("Error sending command to receiver");
                                };

                                if let Err(_) = app_sender.send(Command::UpdateAppBlockchain(
                                    blockchain.clone().as_bytes(),
                                )) {
                                    info!("Error sending updated blockchain to app");
                                }
                            } else {
                                blockchain_network_state.dump(
                                    &block.txns,
                                    block.header.block_reward.clone(),
                                    &block.claims,
                                    block.header.claim.clone(),
                                    &block.hash,
                                );
                                if let Err(_) = miner_sender
                                    .send(Command::ConfirmedBlock(block.clone().as_bytes()))
                                {
                                    info!("Error sending command to receiver");
                                }

                                if let Err(_) = miner_sender.send(Command::StateUpdateCompleted(
                                    blockchain_network_state.clone().as_bytes(),
                                )) {
                                    info!(
                                        "Error sending state update completed command to receiver"
                                    );
                                }

                                if let Err(_) = app_sender.send(Command::UpdateAppBlockchain(
                                    blockchain.clone().as_bytes(),
                                )) {
                                    info!("Error sending blockchain update to App receiver.")
                                }
                            }
                        }
                    }
                    Command::GetStateComponents(requestor, components_bytes, sender_id) => {
                        info!("Received request for State update");
                        let components = StateComponent::from_bytes(&components_bytes);
                        match components {
                            StateComponent::All => {
                                let genesis_bytes =
                                    if let Some(genesis) = blockchain.clone().genesis {
                                        Some(genesis.clone().as_bytes())
                                    } else {
                                        None
                                    };
                                let child_bytes = if let Some(block) = blockchain.clone().child {
                                    Some(block.clone().as_bytes())
                                } else {
                                    None
                                };
                                let parent_bytes = if let Some(block) = blockchain.clone().parent {
                                    Some(block.clone().as_bytes())
                                } else {
                                    None
                                };
                                let current_ledger = Some(
                                    blockchain_network_state.clone().db_to_ledger().as_bytes(),
                                );
                                let current_network_state =
                                    Some(blockchain_network_state.clone().as_bytes());
                                let components = Components {
                                    genesis: genesis_bytes,
                                    child: child_bytes,
                                    parent: parent_bytes,
                                    blockchain: None,
                                    ledger: current_ledger,
                                    network_state: current_network_state,
                                    archive: None,
                                };

                                if let Err(e) = state_sender.send(Command::RequestedComponents(
                                    requestor,
                                    components.as_bytes(),
                                    sender_id.clone(),
                                    blockchain_node_id.clone()
                                )) {
                                    info!(
                                        "Error sending requested components to state receiver: {:?}",
                                        e
                                    );
                                }
                            }
                            _ => {}
                        }
                    }
                    Command::StoreStateComponents(component_bytes, component_type) => {
                        if blockchain.updating_state {
                            blockchain.components_received.insert(component_type.clone());
                            match component_type {
                                ComponentTypes::All => {
                                    let components = Components::from_bytes(&component_bytes);
                                    info!("Received Components: {:?}", components);
                                    if let Some(bytes) = components.genesis {
                                        let genesis = block::Block::from_bytes(&bytes);
                                        blockchain.genesis = Some(genesis);
                                        info!("Stored Genesis: {:?}", blockchain.genesis);
                                    }
                                    if let Some(bytes) = components.child {
                                        let child = block::Block::from_bytes(&bytes);
                                        blockchain.child = Some(child);
                                        info!("Stored child: {:?}", blockchain.child);

                                    }
                                    if let Some(bytes) = components.parent {
                                        let parent = block::Block::from_bytes(&bytes);
                                        blockchain.parent = Some(parent);
                                        info!("Stored parent: {:?}", blockchain.parent);

                                    }
                                    if let Some(bytes) = components.network_state {
                                        if let Ok(mut new_network_state) = NetworkState::from_bytes(component_bytes) {
                                            new_network_state.path = blockchain_network_state.path;
                                            blockchain_reward_state = new_network_state.reward_state.unwrap();
                                            blockchain_network_state = new_network_state;
                                            info!("Stored network state: {:?}", blockchain_network_state);
                                        }
                                    }
                                    if let Some(bytes) = components.ledger {
                                        let new_ledger = Ledger::from_bytes(bytes);
                                        blockchain_network_state.update_ledger(new_ledger);
                                        info!("Stored ledger: {:?}", blockchain_network_state.ledger);
                                    }

                                    info!("Received all core components");
                                    blockchain.updating_state = false;
                                    if let Err(e) = blockchain_sender.send(Command::ProcessBacklog) {
                                        info!("Error sending process backlog command to blockchain receiver: {:?}", e);
                                    }
                                    blockchain.processing_backlog = true;
                                    if let Err(e) = app_sender
                                        .send(Command::UpdateAppBlockchain(blockchain.clone().as_bytes()))
                                    {
                                        info!("Error sending updated blockchain to app: {:?}", e);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    Command::ProcessBacklog => {
                        if blockchain.processing_backlog {
                            let last_block = blockchain.clone().child.unwrap();
                            while let Some((_, block)) = blockchain.future_blocks.pop_front() {
                                if last_block.header.block_height >= block.header.block_height {
                                    info!("Block already processed, skipping")
                                } else {
                                    info!("Processing backlog block: {:?}", block.header.block_height);
                                    if let Err(e) = blockchain.process_block(
                                        &blockchain_network_state,
                                        &blockchain_reward_state,
                                        &block,
                                    ) {
                                        info!(
                                            "Error trying to process backlogged future blocks: {:?} -> {:?}",
                                            e,
                                            block,
                                        );
                                    } else {
                                        blockchain_network_state.dump(
                                            &block.txns,
                                            block.header.block_reward.clone(),
                                            &block.claims,
                                            block.header.claim.clone(),
                                            &block.hash,
                                        );
                                        info!("Processed and confirmed backlog block: {:?}", block.header.block_height);
                                        if let Err(e) = miner_sender
                                            .send(Command::ConfirmedBlock(block.clone().as_bytes()))
                                        {
                                            info!(
                                                "Error sending confirmed backlog block to miner: {:?}",
                                                e
                                            );
                                        }

                                        if let Err(e) = app_sender.send(Command::UpdateAppBlockchain(
                                            blockchain.clone().as_bytes(),
                                        )) {
                                            info!("Error sending blockchain to app: {:?}", e);
                                        }
                                    }
                                }
                            }
                            info!("Backlog processed");

                            if let Err(e) = miner_sender.send(Command::StateUpdateCompleted(
                                blockchain_network_state.clone().as_bytes(),
                            )) {
                                info!("Error sending updated network state to miner: {:?}", e);
                            }

                            if let Err(e) = app_sender
                                .send(Command::UpdateAppBlockchain(blockchain.clone().as_bytes()))
                            {
                                info!("Error sending updated blockchain to app: {:?}", e);
                            }
                            blockchain.processing_backlog = false;
                        }
                    }
                    Command::StateUpdateCompleted(network_state) => {
                        if let Ok(updated_network_state) = NetworkState::from_bytes(network_state) {
                            blockchain_network_state = updated_network_state;
                        }
                        if let Err(e) = app_sender
                            .send(Command::UpdateAppBlockchain(blockchain.clone().as_bytes()))
                        {
                            info!("Error sending blockchain to app: {:?}", e);
                        }
                    }
                    Command::ClaimAbandoned(pubkey, claim_bytes) => {
                        let claim = Claim::from_bytes(&claim_bytes);
                        blockchain_network_state.abandoned_claim(claim.hash.clone());
                        if let Err(_) =
                            miner_sender.send(Command::ClaimAbandoned(pubkey, claim_bytes))
                        {
                            info!("Error sending claim abandoned command to miner");
                        }
                        if let Err(e) = miner_sender.send(Command::StateUpdateCompleted(
                            blockchain_network_state.clone().as_bytes(),
                        )) {
                            info!("Error sending updated network state to miner: {:?}", e);
                        }

                        if let Err(e) = app_sender
                            .send(Command::UpdateAppBlockchain(blockchain.clone().as_bytes()))
                        {
                            info!("Error sending blockchain to app: {:?}", e);
                        }
                    }
                    Command::SlashClaims(bad_validators) => {
                        blockchain_network_state.slash_claims(bad_validators);
                        if let Err(e) = app_sender
                            .send(Command::UpdateAppBlockchain(blockchain.clone().as_bytes()))
                        {
                            info!("Error sending blockchain to app: {:?}", e);
                        }
                    }
                    Command::NonceUp => {
                        blockchain_network_state.nonce_up();
                    }
                    Command::GetHeight => {
                        info!("Blockchain Height: {}", blockchain.chain.len());
                    }
                    _ => {}
                }
            }
        }
    });

}

fn setup_mining() {
    let mining_wallet = wallet.clone();
    let miner_network_state = network_state.clone();
    let miner_reward_state = reward_state.clone();
    let miner_to_miner_sender = to_miner_sender.clone();
    let miner_to_blockchain_sender = to_blockchain_sender.clone();
    let miner_to_gossip_sender = to_gossip_tx.clone();
    let miner_to_app_sender = to_app_sender.clone();
    let miner_node_id = node_id.clone();
    std::thread::spawn(move || {
        let mut miner = Miner::start(
            mining_wallet.clone().get_secretkey(),
            mining_wallet.clone().get_pubkey(),
            mining_wallet.clone().get_address(1),
            miner_reward_state,
            miner_network_state,
            0,
        );
        if let Err(_) = miner_to_app_sender
            .clone()
            .send(Command::UpdateAppMiner(miner.as_bytes()))
        {
            info!("Error sending miner to app");
        }
        loop {
            let blockchain_sender = miner_to_blockchain_sender.clone();
            let gossip_sender = miner_to_gossip_sender.clone();
            let miner_sender = miner_to_miner_sender.clone();
            let app_sender = miner_to_app_sender.clone();
            if let Ok(command) = to_miner_receiver.try_recv() {
                match command {
                    Command::SendMessage(src, message) => {

                        // TODO: Replace the below with sending to the correct channel
                        if let Err(e) = gossip_sender.send((src, message)) {
                            info!("Error sending to swarm receiver: {:?}", e);
                        }
                    }
                    Command::StartMiner => {
                        miner.mining = true;
                        if let Err(_) = miner_sender.send(Command::MineBlock) {
                            info!("Error sending mine block command to miner");
                        }
                    }
                    Command::MineBlock => {
                        if miner.mining {
                            if let Some(last_block) = miner.last_block.clone() {
                                if let Some(claim) =
                                    miner.clone().claim_map.get(&miner.clone().claim.pubkey)
                                {
                                    let lowest_pointer = miner.get_lowest_pointer(
                                        last_block.header.next_block_nonce as u128,
                                    );
                                    if let Some((hash, _)) = lowest_pointer.clone() {
                                        if hash == claim.hash.clone() {
                                            let block = miner.mine();
                                            if let Some(block) = block {
                                                let message = MessageType::BlockMessage {
                                                    block: block.clone().as_bytes(),
                                                    sender_id: miner_node_id.clone().to_string(),
                                                };

                                                let msg_id = MessageKey::rand();
                                                let gossip_msg = GossipMessage {
                                                    id: msg_id.inner(),
                                                    data: message.as_bytes(),
                                                    sender: addr.clone(),
                                                };

                                                let head = Header::Gossip;

                                                let msg = Message {
                                                    head,
                                                    msg: gossip_msg.as_bytes().unwrap()
                                                };

                                                miner.mining = false;

                                                // TODO: Replace the below with sending to the correct channel
                                                if let Err(e) = gossip_sender
                                                    .send((addr.clone(), msg))
                                                {
                                                    info!("Error sending SendMessage command to swarm: {:?}", e);
                                                }

                                                if let Err(_) =
                                                    blockchain_sender.send(Command::PendingBlock(
                                                        block.clone().as_bytes(),
                                                        miner_node_id.clone().to_string(),
                                                    ))
                                                {
                                                    info!("Error sending PendingBlock command to blockchain");
                                                }
                                            } else {
                                                if let Err(e) =
                                                    miner_sender.send(Command::MineBlock)
                                                {
                                                    info!(
                                                        "Error sending miner sender MineBlock: {:?}",
                                                        e
                                                    );
                                                }
                                            }
                                        } else {
                                            miner.mining = false;
                                            if let Err(_) =
                                                miner_sender.send(Command::CheckAbandoned)
                                            {
                                                info!("Error sending check abandoned command to miner");
                                            }
                                        }
                                    } else {
                                        if let Err(e) = miner_sender.send(Command::NonceUp) {
                                            info!(
                                                "Error sending NonceUp command to miner: {:?}",
                                                e
                                            );
                                        }
                                    }
                                }
                            } else {
                                if let Err(e) = miner_sender.send(Command::MineGenesis) {
                                    info!("Error sending mine genesis command to miner: {:?}", e);
                                };
                            }
                        }
                    }
                    Command::ConfirmedBlock(block_bytes) => {
                        let block = block::Block::from_bytes(&block_bytes);
                        miner.current_nonce_timer = block.header.timestamp;

                        if let Category::Motherlode(_) = block.header.block_reward.category {
                            info!("*****{:?}*****\n", &block.header.block_reward.category);
                        }
                        miner.last_block = Some(block.clone());
                        block.txns.iter().for_each(|(k, _)| {
                            miner.txn_pool.confirmed.remove(&k.clone());
                        });
                        let mut new_claims = block.claims.clone();
                        new_claims = new_claims
                            .iter()
                            .map(|(k, v)| {
                                return (k.clone(), v.clone());
                            })
                            .collect();
                        new_claims.iter().for_each(|(k, v)| {
                            miner.claim_pool.confirmed.remove(k);
                            miner.claim_map.insert(k.clone(), v.clone());
                        });

                        // Check if the miner's claim nonce changed,
                        // if it did change, make sure that it HAD to change.
                        // If it did have to change (nonce up) and your local claim map is different
                        // nonce up the local claim map until it is in consensus.
                        miner.claim_map.replace(
                            block.header.claim.clone().pubkey,
                            block.header.claim.clone(),
                        );

                        if let Err(_) = app_sender.send(Command::UpdateAppMiner(miner.as_bytes())) {
                            info!("Error sending updated miner to app")
                        }
                    }
                    Command::ProcessTxn(txn_bytes) => {
                        let txn = Txn::from_bytes(&txn_bytes);
                        let txn_validator = miner.process_txn(txn.clone());
                        miner.check_confirmed(txn.txn_id.clone());
                        let message = MessageType::TxnValidatorMessage {
                            txn_validator: txn_validator.as_bytes(),
                            sender_id: miner_node_id.clone(),
                        };

                        let head = Header::Gossip;
                        let msg_id = MessageKey::rand();
                        let gossip_msg = GossipMessage {
                            id: msg_id.inner(),
                            data: message.as_bytes(),
                            sender: addr.clone()
                        };

                        let msg = Message {
                            head,
                            msg: gossip_msg.as_bytes().unwrap()
                        };


                        // TODO: Replace the below with sending to the correct channel
                        if let Err(e) = gossip_sender.send((addr.clone(), msg)) {
                            info!("Error sending SendMessage command to swarm: {:?}", e);
                        }
                        if let Err(_) = app_sender.send(Command::UpdateAppMiner(miner.as_bytes())) {
                            info!("Error sending updated miner to app.")
                        }
                    }
                    Command::ProcessClaim(claim_bytes) => {
                        let claim = Claim::from_bytes(&claim_bytes);
                        miner
                            .claim_pool
                            .confirmed
                            .insert(claim.pubkey.clone(), claim.clone());
                        if let Err(_) = app_sender.send(Command::UpdateAppMiner(miner.as_bytes())) {
                            info!("Error sending updated miner to app")
                        }
                    }
                    Command::ProcessTxnValidator(validator_bytes) => {
                        let validator = TxnValidator::from_bytes(&validator_bytes);
                        miner.process_txn_validator(validator.clone());
                        if let Some(bad_validators) =
                            miner.check_rejected(validator.txn.txn_id.clone())
                        {
                            if let Err(e) =
                                blockchain_sender.send(Command::SlashClaims(bad_validators.clone()))
                            {
                                info!(
                                    "Error sending SlashClaims command to blockchain thread: {:?}",
                                    e
                                );
                            }

                            bad_validators.iter().for_each(|k| {
                                miner.slash_claim(k.to_string());
                            });
                        } else {
                            miner.check_confirmed(validator.txn.txn_id.clone());
                        }

                        if let Err(_) = app_sender.send(Command::UpdateAppMiner(miner.as_bytes())) {
                            info!("Error sending updated miner to app")
                        }
                    }
                    Command::InvalidBlock(_) => {
                        if let Err(e) = miner_sender.send(Command::MineBlock) {
                            info!("Error sending mine block command to miner: {:?}", e);
                        }
                    }
                    Command::StateUpdateCompleted(network_state_bytes) => {
                        if let Ok(updated_network_state) = NetworkState::from_bytes(network_state_bytes) {
                            miner.network_state = updated_network_state.clone();
                            miner.claim_map = miner.network_state.get_claims();
                            miner.mining = true;
                            if let Err(e) = miner_sender.send(Command::MineBlock) {
                                info!("Error sending MineBlock command to miner: {:?}", e);
                            }
                            if let Err(_) = app_sender.send(Command::UpdateAppMiner(miner.as_bytes())) {
                                info!("Error sending updated miner to app")
                            }
                        }
                    }
                    Command::MineGenesis => {
                        if let Some(block) = miner.genesis() {
                            miner.mining = false;
                            miner.last_block = Some(block.clone());
                            let message = MessageType::BlockMessage {
                                block: block.clone().as_bytes(),
                                sender_id: miner_node_id.clone(),
                            };
                            let head = Header::Gossip;
                            let msg_id = MessageKey::rand();
                            let gossip_msg = GossipMessage {
                                id: msg_id.inner(),
                                data: message.as_bytes(),
                                sender: addr.clone()
                            };

                            let msg = Message {
                                head,
                                msg: gossip_msg.as_bytes().unwrap()
                            };
                            // TODO: Replace the below with sending to the correct channel
                            if let Err(e) = gossip_sender.send((addr.clone(), msg)) {
                                info!("Error sending SendMessage command to swarm: {:?}", e);
                            }
                            if let Err(_) = blockchain_sender.send(Command::PendingBlock(
                                block.clone().as_bytes(),
                                miner_node_id.clone(),
                            )) {
                                info!("Error sending to command receiver")
                            }
                            if let Err(_) =
                                app_sender.send(Command::UpdateAppMiner(miner.as_bytes()))
                            {
                                info!("Error sending updated miner to app")
                            }
                        }
                    }
                    Command::SendAddress => {
                        let message = MessageType::ClaimMessage {
                            claim: miner.claim.clone().as_bytes(),
                            sender_id: miner_node_id.clone(),
                        };
                        let head = Header::Gossip;
                        let msg_id = MessageKey::rand();
                        let gossip_msg = GossipMessage {
                            id: msg_id.inner(),
                            data: message.as_bytes(),
                            sender: addr.clone()
                        };

                        let msg = Message {
                            head,
                            msg: gossip_msg.as_bytes().unwrap()
                        };

                        // TODO: Replace the below with sending to the correct channel
                        if let Err(e) = gossip_sender.send((addr.clone(), msg)) {
                            info!("Error sending SendMessage command to swarm: {:?}", e);
                        }
                    }
                    Command::NonceUp => {
                        miner.nonce_up();
                        if let Err(e) = blockchain_sender.send(Command::NonceUp) {
                            info!("Error sending NonceUp command to blockchain: {:?}", e);
                        }
                        if let Err(_) = app_sender.send(Command::UpdateAppMiner(miner.as_bytes())) {
                            info!("Error sending updated miner to app")
                        }
                        if let Err(e) = miner_sender.send(Command::MineBlock) {
                            info!("Error sending MineBlock command to miner: {:?}", e);
                        }
                    }
                    Command::CheckAbandoned => {
                        if let Some(last_block) = miner.last_block.clone() {
                            if let Some(_) =
                                miner.clone().claim_map.get(&miner.clone().claim.pubkey)
                            {
                                let lowest_pointer = miner
                                    .get_lowest_pointer(last_block.header.next_block_nonce as u128);
                                if let Some((hash, _)) = lowest_pointer.clone() {
                                    if miner.check_time_elapsed() > 30 {
                                        miner.current_nonce_timer = miner.get_timestamp();
                                        let mut abandoned_claim_map = miner.claim_map.clone();
                                        abandoned_claim_map.retain(|_, v| v.hash == hash);

                                        if let Some((_, v)) = abandoned_claim_map.front() {
                                            let message = MessageType::ClaimAbandonedMessage {
                                                claim: v.clone().as_bytes(),
                                                sender_id: miner_node_id.clone(),
                                            };

                                            miner
                                                .abandoned_claim_counter
                                                .insert(miner.claim.pubkey.clone(), v.clone());

                                            let head = Header::Gossip;
                                            let msg_id = MessageKey::rand();
                                            let gossip_msg = GossipMessage {
                                                id: msg_id.inner(),
                                                data: message.as_bytes(),
                                                sender: addr.clone()
                                            };

                                            let msg = Message {
                                                head,
                                                msg: gossip_msg.as_bytes().unwrap()
                                            };
                                            // TODO: Replace the below with sending to the correct channel
                                            if let Err(e) =
                                                gossip_sender.send((addr.clone(), msg))
                                            {
                                                info!("Error sending ClaimAbandoned message to swarm: {:?}", e);
                                            }

                                            let mut abandoned_claim_map =
                                                miner.abandoned_claim_counter.clone();

                                            abandoned_claim_map
                                                .retain(|_, claim| v.hash == claim.hash);

                                            if abandoned_claim_map.len() as f64
                                                / (miner.claim_map.len() as f64 - 1.0)
                                                > VALIDATOR_THRESHOLD
                                            {
                                                miner.claim_map.retain(|_, v| v.hash != hash);
                                                if let Err(e) =
                                                    blockchain_sender.send(Command::ClaimAbandoned(
                                                        miner.claim.pubkey.clone(),
                                                        v.clone().as_bytes(),
                                                    ))
                                                {
                                                    info!("Error forwarding confirmed abandoned claim to blockchain: {:?}", e);
                                                }
                                            }
                                        }
                                    } else {
                                        if let Err(_) = miner_sender.send(Command::CheckAbandoned) {
                                            info!("Error sending check abandoned command to miner");
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Command::ClaimAbandoned(pubkey, _) => {
                        if let Some(claim) = miner.claim_map.clone().get(&pubkey) {
                            miner
                                .abandoned_claim_counter
                                .insert(pubkey.clone(), claim.clone());

                            let mut abandoned_claim_map = miner.abandoned_claim_counter.clone();
                            abandoned_claim_map.retain(|_, v| v.hash == claim.hash);

                            if abandoned_claim_map.len() as f64
                                / (miner.claim_map.len() as f64 - 1.0)
                                > VALIDATOR_THRESHOLD
                            {
                                miner.claim_map.retain(|_, v| v.hash != claim.hash);
                            }
                            if let Err(_) =
                                app_sender.send(Command::UpdateAppMiner(miner.as_bytes()))
                            {
                                info!("Error sending updated miner to app")
                            }
                            miner.mining = true;
                            if let Err(e) = miner_sender.send(Command::MineBlock) {
                                info!("Error sending miner sender MineBlock: {:?}", e);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    });
}

fn state_sending_thread() {
    let state_to_swarm_sender = to_swarm_sender.clone();
    let state_to_gossip_sender = to_gossip_tx.clone();
    let state_to_blockchain_sender = to_blockchain_sender.clone();
    let state_node_id = node_id.clone();
    std::thread::spawn(move || loop {
        let blockchain_sender = state_to_blockchain_sender.clone();
        let swarm_sender = state_to_swarm_sender.clone();
        let gossip_sender = state_to_gossip_sender.clone();
        if let Ok(command) = to_state_receiver.try_recv() {
            match command {
                Command::SendStateComponents(requestor, component_bytes, sender_id) => {
                    let command = Command::GetStateComponents(requestor, component_bytes, sender_id);
                    if let Err(e) = blockchain_sender.send(command) {
                        info!("Error sending component request to blockchain thread: {:?}", e);
                    }
                }
                Command::StoreStateComponents(data, component_type) => {
                    if let Err(e) = blockchain_sender.send(Command::StoreStateComponents(data, component_type)) {
                        info!("Error sending component to blockchain")
                    }
                }
                Command::RequestedComponents(requestor, components, sender_id, requestor_id) => {
                    let restructured_components = Components::from_bytes(&components);
                    let head = Header::Gossip;
                    let message = MessageType::StateComponentsMessage {
                        data: restructured_components.as_bytes(),
                        requestor: requestor.clone(),
                        requestor_id: requestor_id,
                        sender_id: sender_id
                    };

                    let msg_id = MessageKey::rand();
                    let gossip_msg = GossipMessage {
                        id: msg_id.inner(),
                        data: message.as_bytes(),
                        sender: addr.clone()
                    };

                    let msg = Message {
                        head,
                        msg: gossip_msg.as_bytes().unwrap()
                    };

                    let requestor_addr: SocketAddr = requestor.parse().expect("Unable to parse address");

                    match requestor_addr {
                        SocketAddr::V4(v4) => {
                            info!("Requestor is a v4 IP");
                            let ip = v4.ip().clone();
                            let port = 19291;
                            let new_addr = SocketAddrV4::new(ip, port);
                            let tcp_addr = SocketAddr::from(new_addr);
                            match std::net::TcpStream::connect(new_addr) {
                                Ok(mut stream) => {
                                    info!("Opened TCP stream and connected to requestor");
                                    let msg_bytes = msg.as_bytes().unwrap();
                                    let n_bytes = msg_bytes.len();
                                    stream.write(&msg_bytes).unwrap();
                                    info!("Wrote {:?} bytes to tcp stream for requestor", n_bytes);
                                    stream.shutdown(std::net::Shutdown::Both).expect("Unable to shutdown");
                                }
                                Err(_) => {}
                            }
                        }
                        SocketAddr::V6(v6) => {}
                    }


                }
                _ => {
                    info!("Received State Command: {:?}", command);
                }
            }
        }
    });

}

fn swarm_setup() {
    let pub_ip = public_ip::addr_v4().await;
    // let port: usize = thread_rng().gen_range(9292..19292);
    let port: usize = 19292;
    let addr = format!("{:?}:{:?}", pub_ip.clone().unwrap(), port.clone());
    let local_sock: std::net::SocketAddr = addr.parse().expect("unable to parse address");
    // Bind a UDP Socket to a Socket Address with a random port between
    // 9292 and 19292 on the localhost address.
    let sock: UdpSocket = UdpSocket::bind(format!("0.0.0.0:{:?}", port.clone())).expect("Unable to bind to address");



    // // Initialize local peer information
    let key: Key = Key::rand();
    let id: PeerId = PeerId::from_key(&key);
    let info: PeerInfo = PeerInfo::new(id, key, pub_ip.clone().unwrap(), port as u32);

    // // initialize a kademlia, transport and message handler instance
    let routing_table = RoutingTable::new(info.clone());
    let ping_pong = Instant::now();
    let interval = Duration::from_secs(20);
    let kad = Kademlia::new(routing_table, to_transport_tx.clone(), to_kad_rx, HashSet::new(), interval, ping_pong.clone());
    let mut transport = Transport::new(local_sock.clone(), incoming_ack_rx, to_transport_rx);
    let mut message_handler = GossipMessageHandler::new(
        to_transport_tx.clone(),
        incoming_ack_tx.clone(),
        HashMap::new(),
        to_kad_tx.clone(),
        to_gossip_tx.clone(),
    );
    let protocol_id = String::from("vrrb-0.1.0-test-net");
    let gossip_config = GossipConfig::new(
        protocol_id,
        8,
        3,
        8,
        3,
        12,
        3,
        0.4,
        Duration::from_millis(250),
        80,
    );
    let heartbeat = Instant::now();
    let ping_pong = Instant::now();
    let mut gossip = GossipService::new(
        local_sock.clone(),
        info.address.clone(),
        to_gossip_rx,
        to_transport_tx.clone(),
        to_app_tx.clone(),
        kad,
        gossip_config,
        heartbeat,
        ping_pong,
    );

    let thread_sock = sock.try_clone().expect("Unable to clone socket");
    let addr = local_sock.clone();
    std::thread::spawn(move || {
        let inner_sock = thread_sock.try_clone().expect("Unable to clone socket");
        std::thread::spawn(move || loop {
            transport.incoming_ack();
            transport.outgoing_msg(&inner_sock);
            transport.check_time_elapsed(&inner_sock);
        });

        loop {
            let local = addr.clone();
            let mut buf = [0u8; 655360];
            message_handler.recv_msg(&thread_sock, &mut buf, addr.clone());
        }
    });


    if let Some(to_dial) = args().nth(1) {
        let bootstrap: SocketAddr = to_dial.parse().expect("Unable to parse address");
        gossip.kad.bootstrap(&bootstrap);
        if let Some(bytes) = info.as_bytes() {
            gossip.kad.add_peer(bytes)
        }
    } else {
        if let Some(bytes) = info.as_bytes() {
            gossip.kad.add_peer(bytes)
        }
    }

    let thread_to_gossip = to_gossip_tx.clone();
    let (chat_tx, chat_rx) = channel::<GossipMessage>();
    let thread_node_id = node_id.clone();
    let msg_to_command_sender = command_sender.clone();
    std::thread::spawn(move || {
        loop {
            match chat_rx.recv() {
                Ok(gossip_msg) => {
                    if let Some(msg) = MessageType::from_bytes(&gossip_msg.data) {
                        if let Some(command) = message::process_message(msg, thread_node_id.clone(), addr.to_string()) {
                            if let Err(e) = msg_to_command_sender.send(command) {
                                info!("Error sending to command handler: {:?}", e);
                            }
                        }
                    }
                },
                Err(_) => {}
            }
        }
    });

    std::thread::spawn(move || {
        gossip.start(chat_tx.clone());
    });

    info!("Started gossip service");
}

fn setup_node() {
    let to_message_handler = MessageHandler::new(from_message_sender.clone(), to_message_receiver);
    let command_handler = CommandHandler::new(
        to_miner_sender.clone(),
        to_blockchain_sender.clone(),
        to_gossip_sender.clone(),
        to_swarm_sender.clone(),
        to_state_sender.clone(),
        to_gossip_tx.clone(),
        command_receiver,
    );

    let mut node = Node::new(node_type.clone(), command_handler, to_message_handler);
    let node_id = node.id.clone();
    let node_key = node.pubkey.clone();
}

*/
