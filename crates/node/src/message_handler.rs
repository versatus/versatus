use std::{net::SocketAddr, sync::mpsc::Sender};

/// This module is the primary allocator in the system, it contains the data
/// structures and the methods required to send commands to different parts of
/// the system.
use commands::command::Command;
use messages::message_types::MessageType;
use telemetry::info;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use udp2p::protocol::protocol::Message;

use crate::handler::Handler;

/// The basic structure for allocating messages to the transport layer, and
/// receiving messages to be converted into commands from the transport layer.
#[derive(Debug)]
pub struct MessageHandler<T, V> {
    pub sender: UnboundedSender<T>,
    pub receiver: UnboundedReceiver<V>,
}

impl<T: Clone, V: Clone> MessageHandler<T, V> {
    /// Creates and returns a new message handler.
    pub fn new(sender: UnboundedSender<T>, receiver: UnboundedReceiver<V>) -> MessageHandler<T, V> {
        MessageHandler { sender, receiver }
    }

    //i
    async fn run(&mut self) -> MessageType {
        while let Some (msg) = self.receiver.recv().await {
            let msg = msg.clone();
            let msg = msg.as_message();
            match msg {
                MessageType::TxnMessage { txn, sender_id } => {
                    info!("Received transaction message");
                    let cmd = Command::SendMessage { txn, sender_id };
                    self.sender.send(cmd).unwrap();
                },
                MessageType::TxnValidatorMessage { txn_validator, sender_id } => {
                    info!("Received transaction validator message");
                    let cmd = Command::TxnValidatorMessage { txn_validator, sender_id };
                    self.sender.send(cmd).unwrap();
                },
                MessageType::BlockMessage { block, sender_id } => {
                    info!("Received block message");
                    let cmd = Command::BlockMessage { block, sender_id };
                    self.sender.send(cmd).unwrap();
                },
                MessageType::ClaimMessage { claim, sender_id } => {
                    info!("Received claim message");
                    let cmd = Command::ClaimMessage { claim, sender_id };
                    self.sender.send(cmd).unwrap();
                },
                MessageType::GetNetworkStateMessage {
                    sender_id,
                    requested_from,
                    requestor_address,
                    requestor_node_type,
                    lowest_block,
                    component,
                } => {
                    info!("Received get network state message");
                    let cmd = Command::GetNetworkStateMessage {
                        sender_id,
                        requested_from,
                        requestor_address,
                        requestor_node_type,
                        lowest_block,
                        component,
                    };
                    self.sender.send(cmd).unwrap();
                },
                MessageType::InvalidBlockMessage {
                    block_height,
                    reason,
                    miner_id,
                    sender_id,
                } => {
                    info!("Received invalid block message");
                    let cmd = Command::InvalidBlockMessage {
                        block_height,
                        reason,
                        miner_id,
                        sender_id,
                    };
                    self.sender.send(cmd).unwrap();
                },
                MessageType::DisconnectMessage { sender_id, pubkey } => {
                    info!("Received disconnect message");
                    let cmd = Command::DisconnectMessage { sender_id, pubkey };
                    self.sender.send(cmd).unwrap();
                },
                MessageType::StateComponentsMessage {
                    data,
                    requestor,
                    requestor_id,
                    sender_id,
                } => {
                    info!("Received state components message");
                    let cmd = Command::StateComponentsMessage {
                        data,
                        requestor,
                        requestor_id,
                        sender_id,
                    };
                    self.sender.send(cmd).unwrap();
                },
                MessageType
        
        }        
    }
    //removes itself from the active network
    return MessageType::RemovePeerMessage{peer_id: todo!(), socket_addr: todo!() }
    }
}

impl<T: Clone, V: Clone> Handler<T, V> for MessageHandler<T, V> {
    fn send(&self, command: T) -> Option<T> {
        if let Err(_) = self.sender.send(command.clone()) {
            return None;
        } else {
            return Some(command);
        }
    }

    fn recv(&mut self) -> Option<V> {
        if let Ok(message) = self.receiver.try_recv() {
            return Some(message);
        } else {
            return None;
        }
    }
}



