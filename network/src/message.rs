use crate::components::StateComponent;
use commands::command::Command;
use gossipsub::message_types::{MessageType, StateBlock};
use gossipsub::message::Message;

pub const PROPOSAL_EXPIRATION_KEY: &str = "expires";
pub const PROPOSAL_YES_VOTE_KEY: &str = "yes";
pub const PROPOSAL_NO_VOTE_KEY: &str = "no";

#[allow(unused_variables)]
pub fn process_message(message: Message, node_id: String) -> Option<Command> {
    if let Some(message) = MessageType::from_bytes(
        &message.data
    ) {
        match message.clone() {
            MessageType::TxnMessage { txn, .. } => Some(Command::ProcessTxn(txn)),
            MessageType::BlockMessage {
                block, sender_id, ..
            } => Some(Command::PendingBlock(block, sender_id)),
            MessageType::TxnValidatorMessage { txn_validator, .. } => {
                Some(Command::ProcessTxnValidator(txn_validator))
            }
            MessageType::ClaimMessage { claim, .. } => Some(Command::ProcessClaim(claim)),
            MessageType::GetNetworkStateMessage {
                sender_id,
                requested_from,
                lowest_block,
                component,
                ..
            } => {
                if requested_from == node_id {
                    match StateComponent::from_bytes(&component) {
                        StateComponent::NetworkState => {
                            Some(Command::SendStateComponents(sender_id, component))
                        }
                        StateComponent::Blockchain => {
                            Some(Command::SendStateComponents(sender_id, component))
                        }
                        StateComponent::Ledger => {
                            Some(Command::SendStateComponents(sender_id, component))
                        }
                        StateComponent::All => {
                            Some(Command::SendStateComponents(sender_id, component))
                        }
                        _ => Some(Command::SendState(sender_id, lowest_block)),
                    }
                } else {
                    None
                }
            }
            MessageType::BlockChunkMessage {
                requestor,
                block_height,
                chunk_number,
                total_chunks,
                data,
                ..
            } => {
                if requestor == node_id {
                    return Some(Command::StoreStateDbChunk(
                        StateBlock(block_height).as_bytes(),
                        data,
                        chunk_number as u32,
                        total_chunks as u32,
                    ));
                }
                return None;
            }
            MessageType::NeedGenesisBlock {
                sender_id,
                requested_from,
            } => {
                if requested_from == node_id {
                    return Some(Command::SendGenesis(sender_id));
                }
                return None;
            }
            MessageType::StateComponentsMessage {
                data,
                requestor,
                ..
            } => {
                if requestor == node_id {
                    return Some(Command::StoreStateComponents(
                        data,
                    ));
                }
                None
            }
            MessageType::ClaimAbandonedMessage { claim, sender_id } => {
                return Some(Command::ClaimAbandoned(sender_id, claim))
            }
            MessageType::Identify {
                data,
                pubkey,
            } => { 
                // If node type is bootstrap then share peers with the new node
                // and share the new node with existing peers.
                // Otherwise do nothing.
                return None 
            
            },
            MessageType::NewPeer {
                data,
            } => { 
                //TODO: Initialize hole punching protocol
                return None 
            },
            MessageType::KnownPeers {
                data,
            } => { 
                // for up to max peers initialize the hole punching protocol.
                return None 
            },
            MessageType::FirstHolePunch {
                data,
                pubkey,
            } => { 
                
                // Initialize the handshake protocol
                return None 
            
            },
            MessageType::SecondHolePunch {
                data,
                pubkey,
            } => { 
                // If first hole punch message was already received
                // ignore, otherwise initialize handshake protocol    
                return None 
            },
            MessageType::FinalHolePunch {
                data,
                pubkey,
            } => { 
            
                // If first and/or second hole punch message was received
                // ignore, otherwise initialize handshake protocol.
                return None 
            
            },
            MessageType::InitHandshake {
                data,
                pubkey,
                signature,
            } => { 
                // validate the signature based on the data and pubkey
                // if valid, reciprocate handshake.  
                return None 
            },
            MessageType::ReciprocateHandshake {
                data,
                pubkey,
                signature,
            } => { 
                // Validate the signature based on the data and pubkey
                // if valid complete the handshake.
                return None 
            },
            MessageType::CompleteHandshake {
                data,
                pubkey,
                signature,
            } => { 
                // Check that the handshake was indeed initialized and
                // reciprocated. If so, then Validate the signature based 
                // on data and pubkey, if valid
                // return a complete handshake message
                return None 
            },
            MessageType::Ping {
                data,
                addr,
                timestamp,
            } => { 
                // Return a Pong Message.    
                return None 
            },
            MessageType::Pong {
                data,
                addr,
                timestamp,
            } => { 
                // process and log the pong event as a VRRB network event along with the
                // time that it took for the pong to be received back after the ping was sent.
                return None 
            }
            _ => None,
        }
    } else {
        None
    }
}
