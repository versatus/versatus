use std::{
    collections::{HashMap, HashSet},
    error::Error,
    net::SocketAddr,
};

use commands::command::Command;
use messages::{
    message::Message,
    message_types::MessageType,
    packet::{Packet, Packetize},
};
use secp256k1::Secp256k1;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::net::SocketAddr;
use std::str::FromStr;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Error)]
pub enum NodeError {
    #[error("invalid node type {0} provided")]
    InvalidNodeType(String),

    #[error("{0}")]
    Other(String),
}

//TODO:There needs to be different node types, this is probably not the right variants for
//the node types we will need in the network, needs to be discussed.
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum NodeAuth {
    // Builds a full block archive all blocks and all claims
    Archive,
    // Builds a Block Header archive and stores all claims
    Full,
    // Builds a Block Header and Claim Header archive. Maintains claims owned by this node. Can
    // mine blocks and validate transactions cannot validate claim exchanges.
    Light,
    // Stores last block header and all claim headers
    UltraLight,
    //TODO: Add a key field for the bootstrap node, sha256 hash of key in bootstrap node must ==
    // a bootstrap node key.
    Bootstrap,
}

/// Creating a new enum type called NodeType with three variants, Miner,
/// MasterNode and Regular.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum NodeType {
    /// A Node that can archive, validate and mine tokens
    Full,
    /// Same as `NodeType::Full` but without archiving capabilities
    Light,
    /// Archives all transactions processed in the blockchain
    Archive,
    /// Mining node
    Miner,
    Bootstrap,
    Validator,
}

impl FromStr for NodeType {
    type Err = NodeError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        // TODO: define node types thoroughly
        match s {
            "full" => Ok(NodeType::Full),
            "light" => Ok(NodeType::Light),
            _ => Err(NodeError::InvalidNodeType(s.into())),
        }
    }
}

/// The node contains the data and methods needed to operate a node in the
/// network.
#[allow(dead_code)]
#[derive(Debug)]
pub struct Node {
    /// Every node needs to have a secret key to sign messages, blocks, tx, etc.
    /// for authenticity
    //TODO: Discuss whether we need this here or whether it's redundant.
    pub secret_key: Vec<u8>,
    /// Every node needs to have a public key to have its messages, blocks, tx,
    /// etc, signatures validated by other nodes
    //TODOL: Discuss whether this is needed here.
    pub pubkey: String,
    /// Every node needs a unique ID to identify it as a member of the network.
    pub id: String,
    /// The type of the node, used for custom impl's based on the type the
    /// capabilities may vary.
    //TODO: Change this to a generic that takes anything that implements the NodeAuth trait.
    //TODO: Create different custom structs for different kinds of nodes with different
    // authorization so that we can have custom impl blocks based on the type.
    pub node_type: NodeType,
    /// A set of message IDs to check new messages against to prevent redundant
    /// message processing
    //TODO: Move this to the udp2p layer to be handled upon the receipt of messages, rather than
    // by the node itself.
    pub message_cache: HashSet<String>,
    /// Stores packets to be reassembled into a message when all packets are
    /// received
    //TODO: Move this to the udp2p layer to be handled upon the receipt of the message. Node
    // should only receive assembled messages.
    pub packet_storage: HashMap<String, HashMap<u32, Packet>>,
    /// The command handler used to allocate commands to different parts of the
    /// system
    pub command_handler: CommandHandler,
    /// The message handler used to convert received messages into a command and
    /// to structure and pack outgoing messages to be send to the transport
    /// layer.
    pub message_handler: MessageHandler<MessageType, (Packet, SocketAddr)>,

    //Index num of the node in the network
    pub idx: u16,
}

impl Node {
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

    /// Creates and returns a Node instance
    pub fn new(
        node_type: NodeType,
        command_handler: CommandHandler,
        message_handler: MessageHandler<MessageType, (Packet, SocketAddr)>,
        idx: u16,
    ) -> Node {
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
        }*/

        Node {
            secret_key: secret_key_encoded,
            pubkey: pubkey.to_string(),
            id,
            node_type,
            message_cache: HashSet::new(),
            packet_storage: HashMap::new(),
            command_handler,
            message_handler,
            idx,
        }
    }

    /// Handles an incoming packet
    //TODO: Move this to the transport layer, the Node should only deal with messages and commands
    #[allow(unused)]
    pub fn handle_packet(&mut self, packet: &Packet) {
        let packet_number = usize::from_be_bytes(packet.clone().convert_packet_number()) as u32;
        let id = String::from_utf8_lossy(&packet.clone().id).to_string();
        if !self.message_cache.contains(&id) {
            if let Some(map) = self.packet_storage.get_mut(&id) {
                map.insert(packet_number, packet.clone());
                if let Ok(message_bytes) = Message::try_assemble(map) {
                    self.message_cache.insert(id.clone());
                    let message = Message::from_bytes(&message_bytes);
                    let clean_inbox = Command::CleanInbox(id.clone());
                    self.command_handler.handle_command(clean_inbox);
                    // if let Some(command) =
                    //     message::process_message(message, self.id.clone().to_string())
                    // {
                    //     self.command_handler.handle_command(command);
                    // };

                    self.packet_storage.remove(&id.clone());
                }
            } else {
                let mut map = HashMap::new();
                map.insert(packet_number, packet.clone());
                self.packet_storage.insert(id.clone(), map.clone());
                if let Ok(message_bytes) = Message::try_assemble(&mut map) {
                    self.message_cache.insert(id.clone());
                    let message = Message::from_bytes(&message_bytes);
                    let clean_inbox = Command::CleanInbox(id.clone());
                    self.command_handler.handle_command(clean_inbox);

                    // if let Some(command) =
                    //     message::process_message(message, self.id.clone().to_string())
                    // {
                    //     self.command_handler.handle_command(command);
                    // };
                    self.packet_storage.remove(&id.clone());
                }
            }
        }
    }

    /// Starts the program loop for the Node, checking whether there is a
    /// command or message received, and allocating the command/message to
    /// where it needs to go.
    pub async fn start(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            let evt = {
                tokio::select! {
                    command = self.command_handler.receiver.recv() => {
                        if let Some(command) = command {
                            Some(command)
                        } else {
                            None
                        }
                    }
                    from_message = self.message_handler.receiver.recv() => {
                        if let Some((packet, src)) = from_message {
                            Some(Command::ProcessPacket((packet, src)))
                        } else {
                            None
                        }
                    }
                }
            };

            if let Some(command) = evt {
                match command {
                    Command::ProcessPacket((packet, _)) => {
                        self.handle_packet(&packet);
                    },
                    Command::SendMessage(src, message) => {
                        if let Err(e) = self.command_handler.to_gossip_tx.send((src, message)) {
                            println!("Error publishing: {:?}", e);
                        }
                    },
                    Command::Quit => {
                        //TODO:
                        // 1. Inform peers. DONE
                        // 2. Before Ok(()) at the end of this method
                        //    be sure to join all the threads in this method by setting them to
                        // variables    and winding them down at the end
                        // after exiting this event loop. 3. Print out the
                        // node's wallet secret key, the state db filepath and the
                        //    block archive filepath so users can restore their wallet and state
                        //    when rejoining.

                        break;
                    },
                    Command::SendAddress => {
                        if let Err(e) = self
                            .command_handler
                            .to_mining_sender
                            .send(Command::SendAddress)
                        {
                            println!("Error sending SendAddress command to miner: {:?}", e);
                        }
                    },
                    Command::MineBlock => {
                        if let Err(e) = self
                            .command_handler
                            .to_mining_sender
                            .send(Command::StartMiner)
                        {
                            println!("Error sending mine block command to mining thread: {:?}", e);
                        }
                    },
                    Command::SendState(requested_from, lowest_block) => {
                        if let Err(e) = self
                            .command_handler
                            .to_blockchain_sender
                            .send(Command::SendState(requested_from, lowest_block))
                        {
                            println!("Error sending state request to blockchain thread: {:?}", e);
                        }
                    },
                    Command::StoreStateDbChunk(object, data, chunk_number, total_chunks) => {
                        if let Err(e) = self.command_handler.to_blockchain_sender.send(
                            Command::StoreStateDbChunk(object, data, chunk_number, total_chunks),
                        ) {
                            println!(
                                "Error sending StoreStateDbChunk to blockchain thread: {:?}",
                                e
                            );
                        }
                    },
                    _ => {
                        self.command_handler.handle_command(command);
                    },
                }
            } else {
                continue;
            }
        }

        Ok(())
    }
}

impl NodeAuth {
    /// Serializes the NodeAuth variant it is called on into a vector of bytes.
    pub fn as_bytes(&self) -> Vec<u8> {
        serde_json::to_string(self).unwrap().as_bytes().to_vec()
    }
}
