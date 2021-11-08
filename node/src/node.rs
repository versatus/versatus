use crate::handler::{CommandHandler, MessageHandler};
use commands::command::Command;
use log::info;
use messages::message::Message;
use messages::message_types::MessageType;
use messages::packet::{Packet, Packetize};
use network::message;
use secp256k1::Secp256k1;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use uuid::Uuid;

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum NodeAuth {
    // Builds a full block archive all blocks and all claims
    Archive,
    // Builds a Block Header archive and stores all claims
    Full,
    // Builds a Block Header and Claim Header archive. Maintains claims owned by this node. Can mine blocks and validate transactions
    // cannot validate claim exchanges.
    Light,
    // Stores last block header and all claim headers
    UltraLight,
    //TODO: Add a key field for the bootstrap node, sha256 hash of key in bootstrap node must == a bootstrap node key.
    Bootstrap,
}

#[allow(dead_code)]
pub struct Node {
    secret_key: String,
    pub pubkey: String,
    pub id: String,
    pub node_type: NodeAuth,
    pub message_cache: HashMap<String, Vec<u8>>,
    pub packet_storage: HashMap<String, HashMap<u32, Packet>>,
    pub command_handler: CommandHandler,
    pub message_handler: MessageHandler<MessageType, Vec<u8>>,
}

impl Node {
    pub fn get_id(&self) -> String {
        self.id.clone()
    }

    pub fn get_node_type(&self) -> NodeAuth {
        self.node_type.clone()
    }

    pub fn new(
        node_type: NodeAuth,
        command_handler: CommandHandler,
        message_handler: MessageHandler<MessageType, Vec<u8>>,
    ) -> Node {
        let secp = Secp256k1::new();
        let mut rng = rand::thread_rng();
        let (secret_key, pubkey) = secp.generate_keypair(&mut rng);
        let id = Uuid::new_v4().to_simple().to_string();

        Node {
            secret_key: secret_key.to_string(),
            pubkey: pubkey.to_string(),
            id,
            node_type,
            message_cache: HashMap::new(),
            packet_storage: HashMap::new(),
            command_handler,
            message_handler,
        }
    }

    pub fn handle_packet(&mut self, packet: &Packet) {
        let packet_number = usize::from_be_bytes(packet.clone().convert_packet_number()) as u32;
        let total_packets = usize::from_be_bytes(packet.clone().convert_total_packets());
        info!("Received packet {} of {}", &packet_number, &total_packets);
        let id = String::from_utf8_lossy(&packet.clone().id).to_string();
        if let Some(map) = self.packet_storage.get_mut(&id) {
            map.insert(packet_number, packet.clone());
            if let Ok(message_bytes) = Message::try_assemble(map) {
                let message = Message::from_bytes(&message_bytes);
                if let Err(e) = self.command_handler.to_swarm_sender.send(Command::CleanInbox(id.clone())) {
                    info!("Error sending clean inbox command to gossip: {:?}", e);
                }
                if let Some(command) =
                    message::process_message(message, self.id.clone().to_string())
                {
                    self.command_handler.handle_command(command);
                };

                self.packet_storage.remove(&id.clone());
            }
        } else {
            let mut map = HashMap::new();
            map.insert(packet_number, packet.clone());
            self.packet_storage.insert(id.clone(), map.clone());
            if let Ok(message_bytes) = Message::try_assemble(&mut map) {
                let message = Message::from_bytes(&message_bytes);
                if let Some(command) =
                    message::process_message(message, self.id.clone().to_string())
                {
                    self.command_handler.handle_command(command);
                };
                self.packet_storage.remove(&id.clone());
            }
        }
    }

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
                        if let Some(message) = from_message {
                            Some(Command::ProcessPacket(message))
                        } else {
                            None
                        }
                    }
                }
            };
            if let Some(command) = evt {
                match command {
                    Command::ProcessPacket(packet_bytes) => {
                        let inbox = serde_json::from_slice::<HashMap<String, HashMap<u32, Packet>>>(&packet_bytes).unwrap();
                        inbox.iter().for_each(|(_, map)| {
                            map.iter().for_each(|(_, packet)| {
                                self.handle_packet(&packet);
                            });
                        });
                    }
                    Command::SendMessage(message) => {
                        if let Some(message) = MessageType::from_bytes(&message) {
                            if let Err(e) = self
                                .command_handler
                                .to_swarm_sender
                                .send(Command::SendMessage(message.as_bytes()))
                            {
                                println!("Error publishing: {:?}", e);
                            }
                        }
                    }
                    Command::Quit => {
                        //TODO:
                        // 1. Inform peers. DONE
                        // 2. Before Ok(()) at the end of this method
                        //    be sure to join all the threads in this method by setting them to variables
                        //    and winding them down at the end after exiting this event loop.
                        // 3. Print out the node's wallet secret key, the state db filepath and the
                        //    block archive filepath so users can restore their wallet and state
                        //    when rejoining.

                        break;
                    }
                    Command::SendAddress => {
                        if let Err(e) = self
                            .command_handler
                            .to_mining_sender
                            .send(Command::SendAddress)
                        {
                            println!("Error sending SendAddress command to miner: {:?}", e);
                        }
                    }
                    Command::MineBlock => {
                        if let Err(e) = self
                            .command_handler
                            .to_mining_sender
                            .send(Command::StartMiner)
                        {
                            println!("Error sending mine block command to mining thread: {:?}", e);
                        }
                    }
                    Command::SendState(requested_from, lowest_block) => {
                        if let Err(e) = self
                            .command_handler
                            .to_blockchain_sender
                            .send(Command::SendState(requested_from, lowest_block))
                        {
                            println!("Error sending state request to blockchain thread: {:?}", e);
                        }
                    }
                    Command::StoreStateDbChunk(object, data, chunk_number, total_chunks) => {
                        if let Err(e) = self.command_handler.to_blockchain_sender.send(
                            Command::StoreStateDbChunk(object, data, chunk_number, total_chunks),
                        ) {
                            println!(
                                "Error sending StoreStateDbChunk to blockchain thread: {:?}",
                                e
                            );
                        }
                    }
                    _ => {
                        self.command_handler.handle_command(command);
                    }
                }
            } else {
                continue;
            }
        }

        Ok(())
    }
}

impl NodeAuth {
    pub fn as_bytes(&self) -> Vec<u8> {
        serde_json::to_string(self).unwrap().as_bytes().to_vec()
    }
}
