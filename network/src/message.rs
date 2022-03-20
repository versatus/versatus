use crate::components::StateComponent;
use commands::command::Command;
use messages::message_types::{MessageType, StateBlock};
use log::info;

pub const PROPOSAL_EXPIRATION_KEY: &str = "expires";
pub const PROPOSAL_YES_VOTE_KEY: &str = "yes";
pub const PROPOSAL_NO_VOTE_KEY: &str = "no";

#[allow(unused_variables)]
pub fn process_message(message: MessageType, node_id: String) -> Option<Command> {
    info!("Received message to process");
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
                info!("Received state components");
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
        _ => None,
    }
}