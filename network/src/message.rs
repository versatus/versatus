use commands::command::{Command, ComponentTypes};
use log::info;
use messages::message_types::MessageType;

use crate::components::StateComponent;

pub const PROPOSAL_EXPIRATION_KEY: &str = "expires";
pub const PROPOSAL_YES_VOTE_KEY: &str = "yes";
pub const PROPOSAL_NO_VOTE_KEY: &str = "no";

/// Processes messages that come across the network and returns an
/// `Option<Command>` to be allocated to different parts of the system.
#[allow(unused_variables)]
pub fn process_message(message: MessageType, node_id: String, addr: String) -> Option<Command> {
    match message.clone() {
        MessageType::TxnMessage { txn, .. } => Some(Command::ProcessTxn(txn)),
        MessageType::BlockMessage {
            block, sender_id, ..
        } => Some(Command::PendingBlock(block, sender_id)),
        MessageType::TxnValidatorMessage { txn_validator, .. } => {
            Some(Command::ProcessTxnValidator(txn_validator))
        },
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
                    StateComponent::NetworkState => Some(Command::SendStateComponents(
                        requestor_address.to_string(),
                        component,
                        sender_id.clone(),
                    )),
                    StateComponent::Blockchain => Some(Command::SendStateComponents(
                        requestor_address.to_string(),
                        component,
                        sender_id.clone(),
                    )),
                    StateComponent::Ledger => Some(Command::SendStateComponents(
                        requestor_address.to_string(),
                        component,
                        sender_id.clone(),
                    )),
                    StateComponent::All => Some(Command::SendStateComponents(
                        requestor_address.to_string(),
                        component,
                        sender_id.clone(),
                    )),
                    _ => Some(Command::SendState(
                        requestor_address.to_string(),
                        lowest_block,
                    )),
                }
            } else {
                None
            }
        },
        MessageType::StateComponentsMessage {
            data, requestor, ..
        } => {
            info!(
                "Received message to process: {:?} for {:?}",
                message, requestor
            );
            if requestor == node_id {
                info!("Received state components");
                return Some(Command::StoreStateComponents(data, ComponentTypes::All));
            }
            None
        },
        MessageType::GenesisMessage {
            data,
            requestor,
            sender_id,
            requestor_id,
        } => {
            if requestor == addr {
                info!("Received Genesis Block Message");
                Some(Command::StoreStateComponents(data, ComponentTypes::Genesis))
            } else {
                None
            }
        },
        MessageType::ChildMessage {
            data,
            requestor,
            requestor_id,
            sender_id,
        } => {
            if requestor == addr {
                info!("Received Child Block Message");
                Some(Command::StoreStateComponents(data, ComponentTypes::Child))
            } else {
                None
            }
        },
        MessageType::ParentMessage {
            data,
            requestor,
            requestor_id,
            sender_id,
        } => {
            if requestor == addr {
                info!("Received Network Parent Block Message");
                Some(Command::StoreStateComponents(data, ComponentTypes::Parent))
            } else {
                None
            }
        },
        MessageType::LedgerMessage {
            data,
            requestor,
            requestor_id,
            sender_id,
        } => {
            if requestor == addr {
                info!("Received Ledger Message");

                Some(Command::StoreStateComponents(data, ComponentTypes::Ledger))
            } else {
                None
            }
        },
        MessageType::NetworkStateMessage {
            data,
            requestor,
            requestor_id,
            sender_id,
        } => {
            if requestor == addr {
                info!("Received Network State Message");
                Some(Command::StoreStateComponents(
                    data,
                    ComponentTypes::NetworkState,
                ))
            } else {
                None
            }
        },
        MessageType::InvalidBlockMessage {
            block_height,
            reason,
            miner_id,
            sender_id,
        } => {
            if miner_id == node_id {
                // Check the reason, adjust accordingly.
                return Some(Command::StopMine);
            }
            None
        },
        MessageType::ClaimAbandonedMessage { claim, sender_id } => {
            return Some(Command::ClaimAbandoned(sender_id, claim))
        },
        _ => None,
    }
}
