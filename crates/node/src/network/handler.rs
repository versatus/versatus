use std::{
    collections::HashMap,
    net::{AddrParseError, SocketAddr},
    ops::AddAssign,
};

use async_trait::async_trait;
use dyswarm::{
    client::{BroadcastArgs, BroadcastConfig},
    server::ServerConfig,
};
use events::{Event, EventMessage, EventPublisher, EventSubscriber};
use kademlia_dht::{Key, Node as KademliaNode, NodeData};
use primitives::{KademliaPeerId, NodeId, NodeType};
use storage::vrrbdb::VrrbDbReadHandle;
use telemetry::info;
use theater::{Actor, ActorId, ActorImpl, ActorLabel, ActorState, Handler, TheaterError};
use tracing::Subscriber;
use utils::payload::digest_data_to_bytes;
use vrrb_config::{BootstrapQuorumConfig, NodeConfig, QuorumMembershipConfig};
use vrrb_core::claim::Claim;

use super::{NetworkEvent, NetworkModule};
use crate::{
    network::DyswarmHandler, result::Result, NodeError, RuntimeComponent, RuntimeComponentHandle,
    DEFAULT_ERASURE_COUNT,
};

#[async_trait]
impl Handler<EventMessage> for NetworkModule {
    fn id(&self) -> ActorId {
        self.id.clone()
    }

    fn label(&self) -> ActorLabel {
        format!("Network::{}", self.id())
    }

    fn status(&self) -> ActorState {
        self.status.clone()
    }

    fn set_status(&mut self, actor_status: ActorState) {
        self.status = actor_status;
    }

    async fn handle(&mut self, event: EventMessage) -> theater::Result<ActorState> {
        match event.into() {
            Event::PeerJoined(peer_data) => {
                info!("Storing peer information from {} in DHT", peer_data.node_id);

                // TODO: revisit this insert method
                self.kademlia_node.insert(
                    peer_data.kademlia_peer_id,
                    &peer_data.kademlia_liveness_addr.to_string(),
                );

                let evt = Event::NodeAddedToPeerList(peer_data.clone());
                let em = EventMessage::new(Some("consensus-events".into()), evt);

                self.events_tx
                    .send(em)
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },

            Event::QuorumMembershipAssigmentCreated(assigned_membership) => {
                self.notify_quorum_membership_assignment(assigned_membership)
                    .await?;
            },

            Event::ClaimCreated(claim) => {
                info!("Broadcasting claim to peers");
                self.broadcast_claim(claim).await?;
            },

            Event::PartCommitmentCreated(node_id, part) => {
                info!("Broadcasting part commitment to peers in quorum");
                self.broadcast_part_commitment(node_id, part).await?;
            },

            Event::PartCommitmentAcknowledged { node_id, sender_id } => {
                info!("Broadcasting part commitment acknowledgement to peers in quorum");
                self.broadcast_part_commitment_acknowledgement(node_id, sender_id)
                    .await?;
            },

            Event::Stop => {
                // NOTE: stop the kademlia node instance
                self.node_ref().kill();
                return Ok(ActorState::Stopped);
            },
            _ => {},
        }

        Ok(ActorState::Running)
    }

    fn on_stop(&self) {
        info!(
            "{}-{} received stop signal. Stopping",
            self.label(),
            self.id(),
        );
    }
}
