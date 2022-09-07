use std::{
    collections::{HashMap, HashSet},
    error::Error,
    net::SocketAddr,
    str::FromStr,
};

use commands::command::Command;
use messages::{
    message::Message,
    message_types::MessageType,
    packet::{Packet, Packetize},
};
use primitives::{NodeId, NodeIdx};
use secp256k1::Secp256k1;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    command_handler::CommandHandler,
    core::{NodeAuth, NodeType},
    message_handler::MessageHandler,
    result::*,
};

/// The node contains the data and methods needed to operate a node in the
/// network.
#[derive(Debug)]
pub struct Node {
    /// Every node needs a unique ID to identify it as a member of the network.
    pub id: primitives::NodeIdentifier,

    /// Every node needs to have a secret key to sign messages, blocks, tx, etc.
    /// for authenticity
    //TODO: Discuss whether we need this here or whether it's redundant.
    pub secret_key: primitives::SecretKey,

    /// Every node needs to have a public key to have its messages, blocks, tx,
    /// etc, signatures validated by other nodes
    //TODOL: Discuss whether this is needed here.
    pub pubkey: String,

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
    /// to structure and pack outgoing messages to be sent to the transport
    /// layer
    pub message_handler: MessageHandler<MessageType, (Packet, SocketAddr)>,

    /// Index of the node in the network
    pub idx: NodeIdx,
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
    //TODO: Move this to the transport layer, the Node should only deal with
    // messages and commands
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
    #[telemetry::instrument]
    pub async fn start(&mut self) -> Result<()> {
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
                    Command::Stop => {
                        //TODO:
                        // 1. Inform peers. DONE
                        // 2. Before Ok(()) at the end of this method
                        //    be sure to join all the threads in this method by setting them to
                        // variables    and winding them down at the end
                        // after exiting this event loop. 3. Print out the
                        // node's wallet secret key, the state db filepath and the
                        //    block archive filepath so users can restore their wallet and state
                        //    when rejoining.

                        telemetry::info!("Stopping node");
                        break;
                    },
                    #[deprecated(note = "will be removed soon, use Command::Stop instead")]
                    Command::Quit => {},
                    Command::ProcessPacket((packet, _)) => {
                        self.handle_packet(&packet);
                    },
                    Command::SendMessage(src, message) => {
                        if let Err(e) = self.command_handler.to_gossip_tx.send((src, message)) {
                            telemetry::error!("Error publishing: {:?}", e);
                        }
                    },
                    Command::SendAddress => {
                        if let Err(e) = self
                            .command_handler
                            .to_mining_sender
                            .send(Command::SendAddress)
                        {
                            telemetry::error!(
                                "Error sending SendAddress command to miner: {:?}",
                                e
                            );
                        }
                    },
                    Command::MineBlock => {
                        if let Err(e) = self
                            .command_handler
                            .to_mining_sender
                            .send(Command::StartMiner)
                        {
                            telemetry::error!(
                                "Error sending mine block command to mining thread: {:?}",
                                e
                            );
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
                            telemetry::error!(
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
                telemetry::debug!("No message received: {:?}", evt);
                continue;
            }
        }

        Ok(())
    }
}
