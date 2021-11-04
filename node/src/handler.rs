use commands::command::Command;
use log::info;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub trait Handler<T, V> {
    fn send(&self, message: T) -> Option<T>;
    fn recv(&mut self) -> Option<V>;
}

pub struct MessageHandler<T, V> {
    pub sender: UnboundedSender<T>,
    pub receiver: UnboundedReceiver<V>,
}

pub struct CommandHandler {
    pub to_mining_sender: UnboundedSender<Command>,
    pub to_blockchain_sender: UnboundedSender<Command>,
    pub to_swarm_sender: UnboundedSender<Command>,
    pub to_state_sender: UnboundedSender<Command>,
    pub receiver: UnboundedReceiver<Command>,
}

impl<T: Clone, V: Clone> MessageHandler<T, V> {
    pub fn new(sender: UnboundedSender<T>, receiver: UnboundedReceiver<V>) -> MessageHandler<T, V> {
        MessageHandler { sender, receiver }
    }
}

impl CommandHandler {
    pub fn new(
        to_mining_sender: UnboundedSender<Command>,
        to_blockchain_sender: UnboundedSender<Command>,
        to_swarm_sender: UnboundedSender<Command>,
        to_state_sender: UnboundedSender<Command>,
        receiver: UnboundedReceiver<Command>,
    ) -> CommandHandler {
        CommandHandler {
            to_mining_sender,
            to_blockchain_sender,
            to_swarm_sender,
            to_state_sender,
            receiver,
        }
    }

    pub fn handle_command(&mut self, command: Command) {
        match command {
            Command::StopMine => {
                if let Err(e) = self.to_mining_sender.send(Command::StopMine) {
                    println!("Error sending to mining sender: {:?}", e);
                }
            }
            Command::GetState => {
                //TODO: request the state from the most recent confirmed block miner's node.
            }
            Command::ProcessTxn(txn) => {
                if let Err(e) = self.to_mining_sender.send(Command::ProcessTxn(txn)) {
                    println!(
                        "Error sending transaction to mining sender for processing: {:?}",
                        e
                    );
                }
            }
            Command::ProcessTxnValidator(validator) => {
                if let Err(e) = self
                    .to_mining_sender
                    .send(Command::ProcessTxnValidator(validator))
                {
                    println!(
                        "Error sending txn validator to mining sender for processing: {:?}",
                        e
                    );
                }
            }
            Command::ProcessClaim(claim) => {
                if let Err(e) = self.to_mining_sender.send(Command::ProcessClaim(claim)) {
                    println!(
                        "Error sending new claim to mining receiver for processing: {:?}",
                        e
                    );
                }
            }
            Command::StateUpdateCompleted(network_state) => {
                if let Err(e) = self
                    .to_mining_sender
                    .send(Command::StateUpdateCompleted(network_state))
                {
                    println!(
                        "Error sending updated network state to mining receiver: {:?}",
                        e
                    );
                }
            }
            Command::StoreStateDbChunk(_object, _chunk, _chunk_number, _total_chunks) => {}
            Command::ProcessBacklog => {}
            Command::CheckStateUpdateStatus((_block_height, _block, _last_block)) => {}
            Command::Quit => {
                // TODO: Inform all the threads that you're shutting down.
            }
            Command::SendMessage(message) => {
                if let Err(e) = self.to_swarm_sender.send(Command::SendMessage(message)) {
                    println!("Error sending message command to swarm: {:?}", e);
                }
            }
            Command::SendState(_requested_from, _lowest_block) => {}
            Command::SendStateComponents(requested_from, component) => {
                if let Err(e) = self
                    .to_state_sender
                    .send(Command::SendStateComponents(requested_from, component))
                {
                    println!(
                        "Error sending SendStateComponenets Command to state receiver: {:?}",
                        e
                    );
                }
            }
            Command::StoreStateComponents(data) => {
                if let Err(e) = self.to_state_sender.send(Command::StoreStateComponents(
                    data
                )) {
                    println!(
                        "Error sending StoreStateComponentChunk to state receiver: {:?}",
                        e
                    );
                }
            }
            Command::ConfirmedBlock(_block) => {}
            Command::PendingBlock(block, sender_id) => {
                if let Err(e) = self
                    .to_blockchain_sender
                    .send(Command::PendingBlock(block.clone(), sender_id))
                {
                    println!("Error sending pending block to miner: {:?}", e);
                }
            }
            Command::InvalidBlock(_block) => {}
            Command::GetBalance(address) => {
                if let Err(e) = self.to_mining_sender.send(Command::GetBalance(address)) {
                    println!("Error sending GetBalance command to mining thread: {:?}", e);
                }
            }
            Command::SendGenesis(sender_id) => {
                if let Err(e) = self
                    .to_blockchain_sender
                    .send(Command::SendGenesis(sender_id))
                {
                    println!(
                        "Error sending SendGenesis command to blockchain thread: {:?}",
                        e
                    );
                }
            }
            Command::MineGenesis => {}
            Command::GetHeight => {
                if let Err(e) = self.to_blockchain_sender.send(Command::GetHeight) {
                    println!(
                        "Error sending GetHeight command to blockchain thread: {:?}",
                        e
                    );
                }
            }
            Command::MineBlock => {
                info!("Received mine block command, starting the miner");
                if let Err(e) = self.to_mining_sender.send(Command::StartMiner) {
                    println!("Error sending Mine Block command to miner: {:?}", e);
                }
            }
            Command::ClaimAbandoned(sender_id, claim) => {
                if let Err(e) = self
                    .to_mining_sender
                    .send(Command::ClaimAbandoned(sender_id, claim))
                {
                    println!("Error sending claim abandoned command to miner: {:?}", e)
                }
            }
            Command::Bootstrap(new_peer_addr, new_peer_pubkey) => {
                if let Err(e) = self.to_swarm_sender.send(Command::Bootstrap(new_peer_addr, new_peer_pubkey)) {
                    info!("Error sending bootstrap command to swarm: {:?}", e);
                }
            }
            _ => {}
        }
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
