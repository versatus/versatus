use crate::{
    message::AsMessage,
    message_types::MessageType,
    packet::{Packet, Packetize},
};
use commands::command::Command;
use log::info;
use secp256k1::{
    key::{PublicKey, SecretKey},
    Secp256k1,
};
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub const MAX_TRANSMIT_SIZE: usize = 65_507;

#[derive(Debug)]
pub struct GossipServiceConfig {
    ip: String,
    port: u32,
    heartbeat_interval: u32,
    history_length: u32,
    history_gossip: u32,
    ping_frequency: u32,
    flood_enabled: bool,
    max_transmit_size: u32,
    cache_duration: u32,
    gossip_factor: f64,
    pubkey: PublicKey,
    secretkey: SecretKey,
}

#[derive(Debug)]
pub struct GossipService {
    pub sock: UdpSocket,
    // TODO, replace T generic with Command enum or create a Command trait
    // that can be applied to MessageTypes, Packets, etc.
    pub receiver: UnboundedReceiver<Command>,
    pub sender: UnboundedSender<Packet>,
    pub ip: String,
    pub port: u32,
    pub heartbeat_interval: u32,
    pub history_length: u32,
    pub history_gossip: u32,
    pub ping_frequency: u32,
    pub flood_enabled: bool,
    pub max_transmit_size: u32,
    pub cache_duration: u32,
    pub gossip_factor: f64,
    /// Socket Addr -> Peer PubKey so that messages can be decrypted with the peer pubkey in constant time
    pub known_peers: HashMap<SocketAddr, String>,
    pub explicit_peers: HashMap<SocketAddr, String>,
    pub pubkey: PublicKey,
    secretkey: SecretKey,
}

impl GossipService {
    pub fn new(
        config: GossipServiceConfig,
        receiver: UnboundedReceiver<Command>,
        sender: UnboundedSender<Packet>,
    ) -> GossipService {
        GossipService::from_config(config, receiver, sender)
    }

    pub fn from_config(
        config: GossipServiceConfig,
        receiver: UnboundedReceiver<Command>,
        sender: UnboundedSender<Packet>,
    ) -> GossipService {
        let sock =
            UdpSocket::bind(&format!("{}:{}", &config.get_ip(), &config.get_port())).unwrap();
        GossipService {
            sock,
            receiver,
            sender,
            ip: config.get_ip(),
            port: config.get_port(),
            heartbeat_interval: config.get_heartbeat_interval(),
            history_length: config.get_history_length(),
            history_gossip: config.get_history_gossip(),
            ping_frequency: config.get_ping_frequeny(),
            flood_enabled: config.get_flood_enabled(),
            max_transmit_size: config.get_max_transmission_size(),
            cache_duration: config.get_cache_duration(),
            gossip_factor: config.get_gossip_factor(),
            known_peers: HashMap::new(),
            explicit_peers: HashMap::new(),
            pubkey: config.get_pubkey(),
            secretkey: config.get_secret_key(),
        }
    }

    pub fn gossip<T: AsMessage>(&self, message: T) {
        let every_n = 1.0 / self.gossip_factor;
        self.known_peers
            .iter()
            .enumerate()
            .for_each(|(idx, (addr, _))| {
                if idx % every_n as usize == 0 {
                    self.send_packets(addr, message.into_message().as_packet_bytes());
                }
            });
    }

    pub fn publish<T: AsMessage>(&self, message: T) {
        message
            .into_message()
            .as_packet_bytes()
            .iter()
            .for_each(|packet| {
                self.known_peers.iter().for_each(|(addr, _)| {
                    if let Err(e) = self.sock.send_to(packet, addr) {
                        info!(
                            "Error sending packet {:?} to peer {:?}: {:?}",
                            packet, addr, e
                        )
                    }
                });
            })
    }

    pub fn hole_punch<T: AsMessage>(
        &self,
        peer: &SocketAddr,
        first_message: T,
        second_message: T,
        final_message: T,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.sock.set_ttl(32).expect("Unable to set socket ttl");
        self.send_packets(peer, first_message.into_message().as_packet_bytes());
        self.sock.set_ttl(64).expect("Unable to set socket ttl");
        self.send_packets(peer, second_message.into_message().as_packet_bytes());
        self.sock.set_ttl(255).expect("Unable to set socket ttl");
        self.send_packets(peer, final_message.into_message().as_packet_bytes());

        Ok(())
    }

    pub fn init_handshake<T: AsMessage>(&self, peer: &SocketAddr, message: T) {
        self.sock.set_ttl(128).expect("Unable to set socket ttl");
        self.send_packets(peer, message.into_message().as_packet_bytes());
    }

    pub fn reciprocate_handshake<T: AsMessage>(&self, peer: &SocketAddr, message: T) {
        self.sock.set_ttl(128).expect("Unable to set socket ttl");
        self.send_packets(peer, message.into_message().as_packet_bytes());
    }

    pub fn complete_handshake<T: AsMessage>(&self, peer: &SocketAddr, message: T) {
        self.send_packets(peer, message.into_message().as_packet_bytes());
    }

    pub fn send_ping<T: AsMessage>(&self, peer: &SocketAddr, message: T) {
        self.send_packets(peer, message.into_message().as_packet_bytes());
    }

    pub fn return_pong<T: AsMessage>(&self, peer: &SocketAddr, message: T) {
        self.send_packets(peer, message.into_message().as_packet_bytes());
    }

    pub fn prune_peers(&mut self, _peer: &SocketAddr) {
        // TODO: Upon reaching the maximum number of peers,
        // periodically prune the 2 explicit peers that have
        // been explicitly connected the longest to make room
        // for additional peers to connect to you if need be.
    }

    pub fn send_packets(&self, peer: &SocketAddr, packets: Vec<Vec<u8>>) {
        packets.iter().for_each(|packet| {
            if let Err(e) = self.sock.send_to(&packet, peer) {
                info!("Error sending first hole punch message to peer: {:?}", e);
            };
        });
    }

    pub fn start(&mut self) {
        let thread_socket = self.sock.try_clone().unwrap();
        let thread_sender = self.sender.clone();
        std::thread::spawn(move || loop {
            let mut buf = [0; MAX_TRANSMIT_SIZE];
            let (amt, _) = thread_socket.recv_from(&mut buf).expect("no data received");
            if amt > 0 {
                let packet = Packet::from_bytes(&buf[..amt]);
                if let Err(e) = thread_sender.send(packet) {
                    info!("Error forwarding packet to packet processor: {:?}", e);
                }
            }
        });

        loop {
            if let Ok(command) = self.receiver.try_recv() {
                match command {
                    Command::SendMessage(message_bytes) => {
                        let message = MessageType::from_bytes(&message_bytes);
                        if let Some(message_type) = message {
                            self.publish(message_type.clone());
                            let packets = message_type.into_message().as_packet_bytes();
                            info!("n_packets: {:?}", packets.len());
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

impl GossipServiceConfig {
    pub fn get_ip(&self) -> String {
        self.ip.clone()
    }

    pub fn get_port(&self) -> u32 {
        self.port.clone()
    }

    pub fn get_heartbeat_interval(&self) -> u32 {
        self.heartbeat_interval.clone()
    }
    pub fn get_history_length(&self) -> u32 {
        self.history_length.clone()
    }

    pub fn get_history_gossip(&self) -> u32 {
        self.history_gossip.clone()
    }

    pub fn get_ping_frequeny(&self) -> u32 {
        self.ping_frequency.clone()
    }

    pub fn get_flood_enabled(&self) -> bool {
        self.flood_enabled.clone()
    }

    pub fn get_max_transmission_size(&self) -> u32 {
        self.max_transmit_size.clone()
    }

    pub fn get_cache_duration(&self) -> u32 {
        self.cache_duration.clone()
    }

    pub fn get_gossip_factor(&self) -> f64 {
        self.gossip_factor.clone()
    }

    pub fn get_pubkey(&self) -> PublicKey {
        self.pubkey.clone()
    }

    pub fn get_secret_key(&self) -> SecretKey {
        self.secretkey.clone()
    }

    pub fn set_ip(&mut self, pub_ip: String) {
        self.ip = pub_ip
    }

    pub fn set_port(&mut self, port: u32) {
        self.port = port
    }
    pub fn set_heartbeat_interval(&mut self, heartbeat_interval: u32) {
        self.heartbeat_interval = heartbeat_interval
    }

    pub fn set_history_length(&mut self, history_length: u32) {
        self.history_length = history_length
    }

    pub fn set_history_gossip(&mut self, history_gossip: u32) {
        self.history_gossip = history_gossip
    }

    pub fn set_ping_frequency(&mut self, ping_frequency: u32) {
        self.ping_frequency = ping_frequency
    }

    pub fn set_flood_enabled(&mut self, flood_enabled: bool) {
        self.flood_enabled = flood_enabled
    }

    pub fn set_max_transmission_size(&mut self, max_transmit_size: u32) {
        self.max_transmit_size = max_transmit_size
    }

    pub fn set_cache_duration(&mut self, cache_duration: u32) {
        self.cache_duration = cache_duration
    }

    pub fn set_gossip_factor(&mut self, gossip_factor: f64) {
        self.gossip_factor = gossip_factor
    }

    pub fn set_pubkey(&mut self, pubkey: PublicKey) {
        self.pubkey = pubkey
    }

    pub fn set_secret_key(&mut self, secret_key: SecretKey) {
        self.secretkey = secret_key
    }
}

impl Default for GossipServiceConfig {
    fn default() -> GossipServiceConfig {
        let ip = "0.0.0.0".to_string();
        let port = 19292;
        let secp = Secp256k1::new();
        let mut rng = rand::thread_rng();
        let (secretkey, pubkey) = secp.generate_keypair(&mut rng);
        GossipServiceConfig {
            ip,
            port,
            heartbeat_interval: 2,
            history_length: 5,
            history_gossip: 3,
            ping_frequency: 30,
            flood_enabled: true,
            max_transmit_size: 65_507,
            cache_duration: 5,
            gossip_factor: 0.25,
            pubkey: pubkey,
            secretkey: secretkey,
        }
    }
}
