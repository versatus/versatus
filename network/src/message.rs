use crate::components::StateComponent;
use commands::command::Command;
use messages::message_types::{MessageType, StateBlock};
use log::info;

pub const PROPOSAL_EXPIRATION_KEY: &str = "expires";
pub const PROPOSAL_YES_VOTE_KEY: &str = "yes";
pub const PROPOSAL_NO_VOTE_KEY: &str = "no";

#[allow(unused_variables)]
pub fn process_message(message: MessageType, node_id: String) -> Option<Command> {
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
            requestor_address,
            lowest_block,
            component,
            ..
        } => {
            if requested_from == node_id {
                match StateComponent::from_bytes(&component) {
                    StateComponent::NetworkState => {
                        Some(Command::SendStateComponents(requestor_address.to_string(), component))
                    }
                    StateComponent::Blockchain => {
                        Some(Command::SendStateComponents(requestor_address.to_string(), component))
                    }
                    StateComponent::Ledger => {
                        Some(Command::SendStateComponents(requestor_address.to_string(), component))
                    }
                    StateComponent::All => {
                        Some(Command::SendStateComponents(requestor_address.to_string(), component))
                    }
                    _ => Some(Command::SendState(requestor_address.to_string(), lowest_block)),
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
        MessageType::InvalidBlockMessage {
            block_height,
            reason,
            miner_id,
            sender_id,
        } => {
            if miner_id == node_id {
                // Check the reason, adjust accordingly.
                return Some(Command::StopMine)
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
            return Some(Command::Bootstrap(data, pubkey))
        },
        MessageType::NewPeer {
            data,
            pubkey
        } => {
            let addr_string = String::from_utf8_lossy(&data).to_string();
            return Some(Command::AddNewPeer(addr_string, pubkey))
        },
        MessageType::KnownPeers {
            data,
        } => {
            return Some(Command::AddKnownPeers(data))
        },
        MessageType::FirstHolePunch {
            data,
            pubkey,
        } => {
            let addr_string = String::from_utf8_lossy(&data).to_string();
            return Some(Command::InitHandshake(addr_string)) 
        },
        MessageType::SecondHolePunch {
            data,
            pubkey,
        } => {
            let addr_string = String::from_utf8_lossy(&data).to_string();
            return Some(Command::InitHandshake(addr_string))
        },
        MessageType::FinalHolePunch {
            data,
            pubkey,
        } => {       
            let addr_string = String::from_utf8_lossy(&data).to_string();
            return Some(Command::InitHandshake(addr_string))
        },
        MessageType::InitHandshake {
            data,
            pubkey,
            signature,
        } => {
            let addr_string = String::from_utf8_lossy(&data).to_string();
            return Some(Command::ReciprocateHandshake(addr_string, pubkey, signature))
        },
        MessageType::ReciprocateHandshake {
            data,
            pubkey,
            signature,
        } => { 
            let addr_string = String::from_utf8_lossy(&data).to_string();
            return Some(Command::CompleteHandshake(addr_string, pubkey, signature))
        },
        MessageType::CompleteHandshake {
            data,
            pubkey,
            signature,
        } => {
            let addr_string = String::from_utf8_lossy(&data).to_string();
            info!("Received complete handshake from {} validating and if valid, adding explicit peer", &addr_string);
            return Some(Command::AddExplicitPeer(addr_string, pubkey))
        },
        MessageType::Ping {
            data,
            addr,
            timestamp,
        } => {
            let addr_string = String::from_utf8_lossy(&addr).to_string();
            return Some(Command::ReturnPong(data, addr_string))
        },
        MessageType::Pong {
            data,
            addr,
            ping_timestamp,
            pong_timestamp,
        } => { 
            // process and log the pong event as a VRRB network event along with the
            // time that it took for the pong to be received back after the ping was sent.
            return None 
        },
        MessageType::AckMessage {
            packet_id,
            packet_number,
            src,
        } => {
            return Some(Command::ProcessAck(packet_id, packet_number, src));
        }
        _ => None,
    }
}