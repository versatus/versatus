use std::{collections::HashMap, net::SocketAddr};

use block::{Block, Conflict};
use ethereum_types::U256;
use hbbft::crypto::PublicKeySet;
use messr::Router;
use primitives::{
    Address,
    ByteVec,
    FarmerQuorumThreshold,
    HarvesterQuorumThreshold,
    NodeIdx,
    NodeType,
    PeerId,
    QuorumPublicKey,
    QuorumSize,
    QuorumType,
    RawSignature,
};
use quorum::quorum::Quorum;
use serde::{Deserialize, Serialize};
use telemetry::{error, info};
use tokio::sync::{
    broadcast::{self, Receiver},
    mpsc::{Sender, UnboundedReceiver, UnboundedSender},
};
use vrrb_core::{
    claim::Claim,
    txn::{TransactionDigest, Txn},
};

mod event;
mod event_data;
pub use crate::{event::*, event_data::*};

pub const DEFAULT_BUFFER: usize = 1000;

pub type EventRouter = Router<Event>;
pub type EventMessage = messr::Message<Event>;
pub type EventPublisher = Sender<EventMessage>;
pub type EventSubscriber = Receiver<EventMessage>;
pub type Topic = messr::Topic;

#[cfg(test)]
mod tests {
    use super::*;

    fn event_can_turn_into_router_message() {
        let event = Event::NoOp;
        let message: messr::Message<Event> = event.into();

        assert_eq!(message, messr::Message::new(None, Event::NoOp));
    }
}
