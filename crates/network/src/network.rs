

/// This module allows the network to process messages that come across the network as bytes
/// and then allocate them to the proper channel.
use messages::message::Message;
use commands::command::Command;
use thiserror::Error;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::mpsc::unbounded_channel;

use crate::message::process_message;

///Error type for Quorum
#[derive(Error, Debug)]
pub enum InvalidMessage {
    #[error("unable to parse message")]
    MessageParsingError(),

    #[error("unable to match to command")]
    CommandMatchingError(),
}


/*Messages received over the P2P network are raw byte streams, after assembled, 
they are just data structures, nothing has been done with them yet.

In order for these messages to be useful they must be allocated somewhere. 
The VRRB protocol consists of many components, 
each which handle/process different messages (see @Message Processing). 

## Proposed work

A `MessageHandler` that parses, and based on the `MessageType` allocates it to the proper channel. 
This can be tied in with the `MessageProcessor` so that allocation occurs during processing. */

/// The basic structure for allocating commands to different parts of the
/// system.
#[derive(Debug)]
pub struct MessageProcessor {
    pub to_mining_sender: UnboundedSender<Command>,
    pub to_blockchain_sender: UnboundedSender<Command>,
    pub to_gossip_sender: UnboundedSender<Command>,
    pub to_swarm_sender: UnboundedSender<Command>,
    pub to_state_sender: UnboundedSender<Command>,
    pub to_gossip_tx: Sender<(SocketAddr, Message)>,
    pub receiver: UnboundedReceiver<Command>,
}

impl MessageProcessor {
    // Creates and returns a new command handler.
   /*  pub fn new(
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
    } */ }

    pub fn allocate_message(message_bytes: Vec<u8>) -> Result<(), InvalidMessage> {
        let mut message_type;

        if let Some(message) = Message::from_bytes(&message_bytes) {
            //temporary 
            let mut node_id = "0";
            let mut addr = "0";
            if let Some(command) = process_message(message, node_id.to_string(), addr.to_string()){
                //send command to command handler channels
    
            } else {
                return Err(InvalidMessage::CommandMatchingError());
            } 
        }
        else {
            return Err(InvalidMessage::MessageParsingError());
        }
        //temporary, needs to be removed
        let node_id = "0";
        let addr = "0";

        

    }
