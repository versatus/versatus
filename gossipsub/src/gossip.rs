use commands::command::Command;
use gd_udp::gd_udp::GDUdp;
use log::info;
use messages::{
    message::AsMessage,
    message_types::MessageType,
    packet::{Packet, Packetize},
};
use secp256k1::{
    key::{PublicKey, SecretKey},
    Error, Message, Secp256k1, Signature,
};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::net::{SocketAddr, UdpSocket};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc::{UnboundedSender};

pub const MAX_TRANSMIT_SIZE: usize = 65_535;

#[derive(Debug)]
pub struct GossipServiceConfig {
    ip: String,
    port: u32,
    public_addr: Option<String>,
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
    pub gd_udp: GDUdp,
    pub sock: UdpSocket,
    pub to_node_sender: UnboundedSender<(Packet, SocketAddr)>,
    pub ip: String,
    pub public_addr: String,
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
    pub bootstrap: Option<SocketAddr>,
    pub explicit_peers: HashMap<SocketAddr, String>,
    pub pubkey: PublicKey,
    secretkey: SecretKey,
}

impl GossipService {
    pub fn new(
        config: GossipServiceConfig,
        to_node_sender: UnboundedSender<(Packet, SocketAddr)>,
        log: String,
    ) -> GossipService {
        GossipService::from_config(config, to_node_sender, log)
    }

    pub fn from_config(
        config: GossipServiceConfig,
        to_node_sender: UnboundedSender<(Packet, SocketAddr)>,
        log: String,
    ) -> GossipService {
        let addr = format!("{}:{}", &config.get_ip(), &config.get_port());
        let sock = UdpSocket::bind(&addr).unwrap();
        let public_addr = {
            if let Some(pub_addr) = config.get_public_addr() {
                info!("My public address: {:?}", &pub_addr);
                pub_addr
            } else {
                info!("Unable to find public address");
                format!("{}:{}", &config.get_ip(), &config.get_port())
            }
        };
        let gd_udp = GDUdp {
            addr: public_addr,
            buf: Vec::new(),
            buf_cursor: 0,
            outbox: HashMap::new(),
            message_cache: HashSet::new(),
            timer: std::time::Instant::now(),
            log,
        };
        let public_addr = {
            if let Some(addr) = config.get_public_addr() {
                addr
            } else {
                format!("{:?}:{:?}", config.get_ip(), config.get_port())
            }
        };
        GossipService {
            gd_udp,
            sock,
            to_node_sender,
            ip: config.get_ip(),
            port: config.get_port(),
            public_addr,
            heartbeat_interval: config.get_heartbeat_interval(),
            history_length: config.get_history_length(),
            history_gossip: config.get_history_gossip(),
            ping_frequency: config.get_ping_frequeny(),
            flood_enabled: config.get_flood_enabled(),
            max_transmit_size: config.get_max_transmission_size(),
            cache_duration: config.get_cache_duration(),
            gossip_factor: config.get_gossip_factor(),
            known_peers: HashMap::new(),
            bootstrap: None,
            explicit_peers: HashMap::new(),
            pubkey: config.get_pubkey(),
            secretkey: config.get_secret_key(),
        }
    }

    pub fn set_bootstrap(&mut self, addr: SocketAddr) {
        self.bootstrap = Some(addr);
    }

    pub fn ping_bootstrap(&mut self) {
        if let Some(addr) = self.bootstrap {
            let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
            let message = MessageType::Ping {
                data: vec![0,0,0,0],
                addr: self.public_addr.as_bytes().to_vec(),
                timestamp: timestamp.to_be_bytes().to_vec(),
            };
            message.into_message(0).into_packets().iter().for_each(|packet| {
                if let Err(e) = self.sock.send_to(&packet.as_bytes(), addr) {
                    println!("Error sending ping to bootstrap: {:?}", e)
                };
            });
        }
    }

    pub fn gossip<T: AsMessage>(&mut self, message: T) {
        let every_n = 1.0 / self.gossip_factor;
        self.known_peers
            .clone()
            .iter()
            .enumerate()
            .for_each(|(idx, (addr, _))| {
                if idx % every_n as usize == 0 {
                    let packets = message.into_message(1).into_packets();
                    packets.iter().for_each(|packet| {
                        self.gd_udp.send_reliable(addr, packet.clone(), &self.sock);
                    });
                }
            });
    }

    pub fn publish<T: AsMessage>(&mut self, message: T) -> std::io::Result<()> {
        message
            .into_message(1)
            .into_packets()
            .iter()
            .for_each(|packet| {
                self.known_peers.clone().iter().for_each(|(addr, _)| {
                    self.gd_udp.send_reliable(addr, packet.clone(), &self.sock)
                });
            });

        Ok(())
    }

    pub fn hole_punch<T: AsMessage>(
        &mut self,
        peer: &SocketAddr,
        first_message: T,
        second_message: T,
        final_message: T,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.sock.set_ttl(255).expect("Unable to set socket ttl");
        let first_message_packets = first_message.into_message(1).into_packets();
        first_message_packets.iter().for_each(|packet| {
            self.gd_udp.send_reliable(peer, packet.clone(), &self.sock);
        });
        let second_message_packets = second_message.into_message(1).into_packets();
        second_message_packets.iter().for_each(|packet| {
            self.gd_udp.send_reliable(peer, packet.clone(), &self.sock);
        });
        let final_message_packets = final_message.into_message(1).into_packets();
        final_message_packets.iter().for_each(|packet| {
            self.gd_udp.send_reliable(peer, packet.clone(), &self.sock);
        });

        self.sock.set_ttl(64).expect("Unable to set socket ttl");
        Ok(())
    }

    pub fn init_handshake(&mut self, peer: &SocketAddr) {
        let result = self.sign(self.public_addr.clone().as_bytes());
        if let Ok(signature) = result {
            let message = MessageType::InitHandshake {
                data: self.public_addr.clone().as_bytes().to_vec(),
                pubkey: self.pubkey.to_string(),
                signature: signature.to_string(),
            };
            let packets = message.into_message(1).into_packets();
            packets.iter().for_each(|packet| {
                self.gd_udp.send_reliable(&peer, packet.clone(), &self.sock);
            });
        } else {
            info!("Error signing data");
        }
    }

    pub fn reciprocate_handshake(&mut self, peer: &SocketAddr) {
        let result = self.sign(self.public_addr.clone().as_bytes());
        if let Ok(signature) = result {
            let message = MessageType::ReciprocateHandshake {
                data: self.public_addr.clone().as_bytes().to_vec(),
                pubkey: self.pubkey.to_string(),
                signature: signature.to_string(),
            };
            let packets = message.into_message(1).into_packets();
            packets.iter().for_each(|packet| {
                self.gd_udp.send_reliable(&peer, packet.clone(), &self.sock);
            });
        } else {
            info!("Error signing data");
        }
    }

    pub fn complete_handshake(&mut self, peer: &SocketAddr) {
        let result = self.sign(self.public_addr.clone().as_bytes());
        if let Ok(signature) = result {
            let message = MessageType::CompleteHandshake {
                data: self.public_addr.clone().as_bytes().to_vec(),
                pubkey: self.pubkey.to_string(),
                signature: signature.to_string(),
            };
            let packets = message.into_message(1).into_packets();
            packets.iter().for_each(|packet| {
                self.gd_udp.send_reliable(&peer, packet.clone(), &self.sock);
            });
        } else {
            info!("Error signing data");
        }
    }

    pub fn send_ping<T: AsMessage>(&mut self, peer: &SocketAddr, message: T) {
        let packets = message.into_message(1).into_packets();
        packets.iter().for_each(|packet| {
            self.gd_udp.send_reliable(peer, packet.clone(), &self.sock);
        });
    }

    pub fn return_pong<T: AsMessage>(&mut self, peer: &SocketAddr, message: T) {
        let packets = message.into_message(0).into_packets();
        packets.iter().for_each(|packet| {
            self.gd_udp.send_reliable(peer, packet.clone(), &self.sock);
        });
    }

    pub fn prune_peers(&mut self, _peer: &SocketAddr) {
        // TODO: Upon reaching the maximum number of peers,
        // periodically prune the 2 explicit peers that have
        // been explicitly connected the longest to make room
        // for additional peers to connect to you if need be.
    }

    pub fn send_packets(&mut self, peer: &SocketAddr, packets: Vec<Packet>) {
        packets.iter().for_each(|packet| {
            self.gd_udp.send_reliable(peer, packet.clone(), &self.sock);
        });
    }

    pub fn sign(&self, message: &[u8]) -> Result<Signature, Error> {
        let message_hash = blake3::hash(&message);
        let message_hash = Message::from_slice(message_hash.as_bytes())?;
        let secp = Secp256k1::new();
        let sig = secp.sign(&message_hash, &self.secretkey);
        Ok(sig)
    }

    pub fn verify(
        &self,
        message: &[u8],
        signature: Signature,
        pk: PublicKey,
    ) -> Result<bool, Error> {
        let message_hash = blake3::hash(&message);
        let message_hash = Message::from_slice(message_hash.as_bytes())?;
        let secp = Secp256k1::new();
        let valid = secp.verify(&message_hash, &signature, &pk);

        match valid {
            Ok(()) => Ok(true),
            _ => Err(Error::IncorrectSignature),
        }
    }

    pub fn process_gossip_command(&mut self, command: Command) {
        match command {
            Command::SendStateComponents(_, message_bytes) => {
                if let Some(message) =  MessageType::from_bytes(&message_bytes) {
                    if let Err(e) = self.publish(message.clone()) {
                        info!("Error publishing state components: {:?}", e);
                    }
                }
            }
            Command::SendMessage(message) => {
                if let Err(e) = self.publish(message.clone()) {
                    info!("Error publishing message: {:?}", e);
                }
            }
            Command::AddNewPeer(peer_addr, pubkey) => {
                if peer_addr != self.public_addr {
                    let peer_addr: SocketAddr =
                        peer_addr.parse().expect("Cannot parse peer socket address");
                    let first_message = MessageType::FirstHolePunch {
                        data: self.public_addr.as_bytes().to_vec(),
                        pubkey: self.pubkey.to_string().clone(),
                    };
                    let second_message = MessageType::SecondHolePunch {
                        data: self.public_addr.as_bytes().to_vec(),
                        pubkey: self.pubkey.to_string().clone(),
                    };
                    let final_message = MessageType::FinalHolePunch {
                        data: self.public_addr.as_bytes().to_vec(),
                        pubkey: self.pubkey.to_string().clone(),
                    };

                    if let Err(e) =
                        self.hole_punch(&peer_addr, first_message, second_message, final_message)
                    {
                        info!("Error punching hole to peer {:?}", e);
                    };
                    self.known_peers.insert(peer_addr, pubkey);
                }
            }
            Command::AddKnownPeers(data) => {
                let map = serde_json::from_slice::<HashMap<SocketAddr, String>>(&data).unwrap();
                self.known_peers.extend(map.clone());
                let first_message = MessageType::FirstHolePunch {
                    data: self.public_addr.as_bytes().to_vec(),
                    pubkey: self.pubkey.to_string().clone(),
                };
                let second_message = MessageType::SecondHolePunch {
                    data: self.public_addr.as_bytes().to_vec(),
                    pubkey: self.pubkey.to_string().clone(),
                };
                let final_message = MessageType::FinalHolePunch {
                    data: self.public_addr.as_bytes().to_vec(),
                    pubkey: self.pubkey.to_string().clone(),
                };
                let known_peers = self.known_peers.clone();
                known_peers.iter().for_each(|(addr, _)| {
                    if addr.to_string() != self.public_addr {
                        if let Err(e) = self.hole_punch(
                            addr,
                            first_message.clone(),
                            second_message.clone(),
                            final_message.clone(),
                        ) {
                            info!("Error punching hole to peer {:?}", e);
                        };
                    }
                });
            }
            Command::AddExplicitPeer(addr, pubkey) => {
                let addr: SocketAddr = addr.parse().expect("Cannot parse address");
                info!("Completed holepunching and handshaking process, adding new explicit peer: {:?}", &addr);
                self.explicit_peers.insert(addr, pubkey);
            }
            Command::Bootstrap(addr, pubkey) => {
                let addr: SocketAddr = addr.parse().expect("Cannot parse address");
                let mut other_peers = self.known_peers.clone();
                other_peers.retain(|peer_addr, _| peer_addr != &addr);
                let known_peers_message = MessageType::KnownPeers {
                    data: serde_json::to_string(&other_peers.clone())
                        .unwrap()
                        .as_bytes()
                        .to_vec(),
                };

                let packets = known_peers_message.into_message(1).into_packets();
                packets.iter().for_each(|packet| {
                    self.gd_udp.send_reliable(&addr, packet.clone(), &self.sock);
                });
                let new_peer_message = MessageType::NewPeer {
                    data: addr.to_string().as_bytes().to_vec(),
                    pubkey: pubkey.clone(),
                };
                self.publish(new_peer_message).expect("Would block");
                self.known_peers.insert(addr, pubkey.clone());
            }
            Command::InitHandshake(data) => {
                let peer_addr: SocketAddr = data.parse().expect("cannot parse socket address");
                self.init_handshake(&peer_addr)
            }
            Command::ReciprocateHandshake(data, pubkey, signature) => {
                let peer_addr: SocketAddr =
                    data.clone().parse().expect("cannot parse socket address");
                if let Ok(signature) = Signature::from_str(&signature) {
                    if let Ok(pubkey) = PublicKey::from_str(&pubkey) {
                        if let Ok(true) = self.verify(data.clone().as_bytes(), signature, pubkey) {
                            info!("Initial Handshake is valid, reciprocating handshake with peer {:?}", &peer_addr);
                            self.reciprocate_handshake(&peer_addr);
                        } else {
                            info!("Signature validation failed");
                        }
                    } else {
                        info!("Pubkey unable to be converted from str");
                    }
                } else {
                    info!("Signature unable to be converted from str")
                };
            }
            Command::CompleteHandshake(data, pubkey, signature) => {
                let peer_addr: SocketAddr =
                    data.clone().parse().expect("cannot parse socket address");
                if let Ok(signature) = Signature::from_str(&signature) {
                    if let Ok(pubkey) = PublicKey::from_str(&pubkey) {
                        if let Ok(true) = self.verify(data.clone().as_bytes(), signature, pubkey) {
                            self.complete_handshake(&peer_addr);
                        }
                    }
                };
            }
            Command::SendPing(_) => {}
            Command::ReturnPong(_, _) => {}
            Command::ProcessAck(id, packet_number, src) => {
                self.gd_udp.process_ack(id, packet_number, src);
            }
            _ => {}
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

    pub fn get_public_addr(&self) -> Option<String> {
        self.public_addr.clone()
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

    pub fn set_public_addr(&mut self, public_addr: Option<String>) {
        self.public_addr = public_addr
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
            public_addr: None,
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
