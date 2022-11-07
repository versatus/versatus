use std::{net::SocketAddr, sync::mpsc::Sender};

/// This module is the primary allocator in the system, it contains the data
/// structures and the methods required to send commands to different parts of
/// the system.
use commands::command::Command;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use udp2p::protocol::protocol::Message;

struct MessageHeader {
    //
}

struct MessageEnvelope {
    //
}

/// The basic structure for allocating commands to different parts of the
/// system.
#[derive(Debug)]
#[deprecated(note = "replaced by EventRouter")]
pub struct CommandHandler {
    pub to_mining_sender: UnboundedSender<Command>,
    pub to_blockchain_sender: UnboundedSender<Command>,
    pub to_gossip_sender: UnboundedSender<Command>,
    pub to_swarm_sender: UnboundedSender<Command>,
    pub to_state_sender: UnboundedSender<Command>,
    pub to_gossip_tx: Sender<(SocketAddr, Message)>,
    pub receiver: UnboundedReceiver<Command>,
}

impl CommandHandler {
    /// Creates and returns a new command handler.
    pub fn new(
        to_mining_sender: UnboundedSender<Command>,
        to_blockchain_sender: UnboundedSender<Command>,
        to_gossip_sender: UnboundedSender<Command>,
        to_swarm_sender: UnboundedSender<Command>,
        to_state_sender: UnboundedSender<Command>,
        to_gossip_tx: Sender<(SocketAddr, Message)>,
        receiver: UnboundedReceiver<Command>,
    ) -> CommandHandler {
        CommandHandler {
            to_mining_sender,
            to_blockchain_sender,
            to_gossip_sender,
            to_swarm_sender,
            to_state_sender,
            to_gossip_tx,
            receiver,
        }
    }

    /// Handles a command received by the command handler and allocates it to
    /// the proper part of the system for processing.
    #[telemetry::instrument]
    pub fn handle_command(&mut self, command: Command) {
        match command {
            Command::StopMine => {
                if let Err(e) = self.to_mining_sender.send(Command::StopMine) {
                    telemetry::error!("Error sending to mining sender: {:?}", e);
                }
            },
            Command::GetState => {
                //TODO: request the state from the most recent confirmed block
                // miner's node.
            },
            Command::ProcessTxn(txn) => {
                if let Err(e) = self.to_mining_sender.send(Command::ProcessTxn(txn)) {
                    telemetry::error!(
                        "Error sending transaction to mining sender for processing: {:?}",
                        e
                    );
                }
            },
            Command::ProcessTxnValidator(validator) => {
                if let Err(e) = self
                    .to_mining_sender
                    .send(Command::ProcessTxnValidator(validator))
                {
                    telemetry::error!(
                        "Error sending txn validator to mining sender for processing: {:?}",
                        e
                    );
                }
            },
            Command::ProcessClaim(claim) => {
                if let Err(e) = self.to_mining_sender.send(Command::ProcessClaim(claim)) {
                    telemetry::error!(
                        "Error sending new claim to mining receiver for processing: {:?}",
                        e
                    );
                }
            },
            Command::StateUpdateCompleted(network_state) => {
                if let Err(e) = self
                    .to_mining_sender
                    .send(Command::StateUpdateCompleted(network_state))
                {
                    telemetry::error!(
                        "Error sending updated network state to mining receiver: {:?}",
                        e
                    );
                }
            },
            Command::StoreStateDbChunk(_object, _chunk, _chunk_number, _total_chunks) => {},
            Command::ProcessBacklog => {},
            Command::CheckStateUpdateStatus((_block_height, _block, _last_block)) => {},
            Command::Stop => {
                // TODO: Inform all the threads that you're shutting down.
            },
            Command::SendMessage(src, message) => {
                if let Err(e) = self.to_gossip_tx.send((src, message)) {
                    telemetry::error!("Error sending message command to swarm: {:?}", e);
                }
            },
            Command::SendState(_requested_from, _lowest_block) => {},
            Command::SendStateComponents(requested_from, component, sender_id) => {
                if let Err(e) = self.to_state_sender.send(Command::SendStateComponents(
                    requested_from,
                    component,
                    sender_id,
                )) {
                    telemetry::error!(
                        "Error sending SendStateComponenets Command to state receiver: {:?}",
                        e
                    );
                }
            },
            Command::StoreStateComponents(data, component_type) => {
                if let Err(e) = self
                    .to_state_sender
                    .send(Command::StoreStateComponents(data, component_type))
                {
                    telemetry::error!(
                        "Error sending StoreStateComponentChunk to state receiver: {:?}",
                        e
                    );
                }
            },
            Command::ConfirmedBlock(_block) => {},
            Command::PendingBlock(block, sender_id) => {
                if let Err(e) = self
                    .to_blockchain_sender
                    .send(Command::PendingBlock(block.clone(), sender_id))
                {
                    telemetry::error!("Error sending pending block to miner: {:?}", e);
                }
            },
            Command::InvalidBlock(_block) => {},
            Command::GetBalance(address) => {
                if let Err(e) = self.to_mining_sender.send(Command::GetBalance(address)) {
                    telemetry::error!("Error sending GetBalance command to mining thread: {:?}", e);
                }
            },
            Command::SendGenesis(sender_id) => {
                if let Err(e) = self
                    .to_blockchain_sender
                    .send(Command::SendGenesis(sender_id))
                {
                    telemetry::error!(
                        "Error sending SendGenesis command to blockchain thread: {:?}",
                        e
                    );
                }
            },
            Command::MineGenesis => {},
            Command::GetHeight => {
                if let Err(e) = self.to_blockchain_sender.send(Command::GetHeight) {
                    telemetry::error!(
                        "Error sending GetHeight command to blockchain thread: {:?}",
                        e
                    );
                }
            },
            Command::MineBlock => {
                telemetry::info!("Received mine block command, starting the miner");
                if let Err(e) = self.to_mining_sender.send(Command::StartMiner) {
                    telemetry::error!("Error sending Mine Block command to miner: {:?}", e);
                }
            },
            Command::ClaimAbandoned(sender_id, claim) => {
                if let Err(e) = self
                    .to_mining_sender
                    .send(Command::ClaimAbandoned(sender_id, claim))
                {
                    telemetry::error!("Error sending claim abandoned command to miner: {:?}", e)
                }
            },
            _ => {},
        }
    }
}

//
//TODO: Discuss whether we want to keep these, they have largely been
// outsourced to udp2p and will likely make more sense as part of the network
// specific modules.
//
// Command::Bootstrap(new_peer_addr, new_peer_pubkey) => {
//     if let Err(e) =
// self.to_swarm_sender.send(Command::Bootstrap(new_peer_addr, new_peer_pubkey))
// {         info!("Error sending bootstrap command to swarm: {:?}", e);     }
// }
// Command::AddNewPeer(new_peer_addr, new_peer_pubkey) => {
//     if let Err(e) =
// self.to_swarm_sender.send(Command::AddNewPeer(new_peer_addr,
// new_peer_pubkey)) {         info!("Error sending add new peer command to
// swarm: {:?}", e);     }
// }
// Command::AddKnownPeers(data) => {
//     if let Err(e) = self.to_swarm_sender.send(Command::AddKnownPeers(data)) {
//         info!("Error sending add known peers command to swarm: {:?}", e);
//     }
// }
// Command::AddExplicitPeer(peer_addr, peer_pubkey) => {
//     if let Err(e) =
// self.to_swarm_sender.send(Command::AddExplicitPeer(peer_addr, peer_pubkey)) {
// info!("Error sending add explicit peer command to swarm: {:?}", e);     }
// }
// Command::InitHandshake(peer_addr) => {
//     if let Err(e) =
// self.to_swarm_sender.send(Command::InitHandshake(peer_addr)) {         info!
// ("Error sending initialize handshake command to swarm: {:?}", e);     }
// }
// Command::ReciprocateHandshake(peer_addr, pubkey, signature) => {
//     if let Err(e) =
// self.to_swarm_sender.send(Command::ReciprocateHandshake(peer_addr, pubkey,
// signature)) {         info!("Error sending reciprocate handshake command
// to swarm: {:?}", e);     }
// }
// Command::CompleteHandshake(peer_addr, pubkey, signature) => {
//     if let Err(e) =
// self.to_swarm_sender.send(Command::CompleteHandshake(peer_addr,
// pubkey, signature)) {         info!("Error sending complete handshake
// command to swarm: {:?}", e);     }
// }
// Command::ProcessAck(packet_id, packet_number, src) => {
//     if let Err(e) = self.to_swarm_sender.send(Command::ProcessAck(packet_id,
// packet_number, src)) {         info!("Error sending process ack command
// to swarm: {:?}", e);     }
// }
// Command::CleanInbox(id) => {
//     if let Err(e) =
// self.to_swarm_sender.send(Command::CleanInbox(id.clone())) {         info!("
// Error sending process ack command to swarm: {:?}", e);     }
// }
//
