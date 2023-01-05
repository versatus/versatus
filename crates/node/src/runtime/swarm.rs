use std::{
    collections::{HashMap, HashSet},
    env::args,
    fs,
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, SocketAddrV6, UdpSocket},
    sync::mpsc::{channel, Receiver, Sender},
    time::{Duration, Instant},
};

use block::invalid::InvalidBlockErrorReason;
use claim::claim::Claim;
use commands::command::ComponentTypes;
use events::events::{write_to_json, VrrbNetworkEvent};
use ledger::ledger::Ledger;
use miner::miner::Miner;
use network::{components::StateComponent, message};
use public_ip;
use rand::{thread_rng, Rng};
use reward::reward::Reward;
use ritelinked::LinkedHashMap;
use state::{Components, NetworkState};
use telemetry::info;
use tokio::sync::mpsc::{self, error::TryRecvError};
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
use vrrb_core::event_router::Event;
use wallet::wallet::WalletAccount;

use crate::{node_auth::NodeAuth, result::Result, RuntimeModule, RuntimeModuleState};

type Port = usize;

#[derive(Debug)]
pub struct SwarmConfig {
    // pub ip_address: Option<Ipv4Addr>,
    pub port: Port,
    pub ip_address: Ipv4Addr,
    pub local_socket_addr: SocketAddr,
    pub pub_socket_addr: SocketAddr,
    pub udp_socket: UdpSocket,
}

#[derive(Debug, Clone)]
pub struct SwarmModule {
    //
}

impl RuntimeModule for SwarmModule {
    fn name(&self) -> String {
        String::from("Swarm module")
    }

    fn status(&self) -> RuntimeModuleState {
        todo!()
    }

    fn start(&mut self, control_rx: &mut mpsc::UnboundedReceiver<Event>) -> Result<()> {
        // TODO: rethink this loop
        loop {
            match control_rx.try_recv() {
                Ok(sig) => {
                    telemetry::info!("Received stop signal");
                    break;
                },
                Err(err) if err == TryRecvError::Disconnected => {
                    telemetry::warn!("Failed to process stop signal. Reason: {0}", err);
                    telemetry::warn!("{} shutting down", self.name());
                    break;
                },
                _ => {},
            }
        }

        Ok(())
    }
}

impl SwarmModule {
    pub fn new(swarm_config: SwarmConfig) -> Self {
        let pub_ip = swarm_config.ip_address;
        let port = swarm_config.port;

        /*
        let local_sock = swarm_config.local_socket_addr;
        // let sock = swarm_config.public_socket_addr;
        let sock = swarm_config.udp_socket;

        // // Initialize local peer information
        let key: Key = Key::rand();
        let id: PeerId = PeerId::from_key(&key);
        let info: PeerInfo = PeerInfo::new(id, key, pub_ip.clone().unwrap(), port as u32);

        // // initialize a kademlia, transport and message handler instance
        let routing_table = RoutingTable::new(info.clone());
        let ping_pong = Instant::now();
        let interval = Duration::from_secs(20);
        let kad = Kademlia::new(
            routing_table,
            to_transport_tx.clone(),
            to_kad_rx,
            HashSet::new(),
            interval,
            ping_pong.clone(),
        );

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
        std::thread::spawn(move || loop {
            match chat_rx.recv() {
                Ok(gossip_msg) => {
                    if let Some(msg) = MessageType::from_bytes(&gossip_msg.data) {
                        if let Some(command) =
                            message::process_message(msg, thread_node_id.clone(), addr.to_string())
                        {
                            if let Err(e) = msg_to_command_sender.send(command) {
                                info!("Error sending to command handler: {:?}", e);
                            }
                        }
                    }
                },
                Err(_) => {},
            }
        });

        std::thread::spawn(move || {
            gossip.start(chat_tx.clone());
        });

        info!("Started gossip service");

        */

        Self {}
    }
}

// TODO: return a swarm like struct that can be started
pub fn setup_swarm(swarm_config: SwarmConfig) {}
