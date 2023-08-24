use async_trait::async_trait;
use dyswarm::types::Message as DyswarmMessage;
use events::{Event, EventMessage, EventPublisher, PeerData};
use primitives::{NodeId, NodeType};
use vrrb_config::{QuorumMember, QuorumMembershipConfig};

use crate::{network::NetworkEvent, NodeError};

#[derive(Debug, Clone)]
pub struct DyswarmHandler {
    pub node_id: NodeId,
    pub events_tx: EventPublisher,
}

impl DyswarmHandler {
    pub fn new(node_id: NodeId, events_tx: EventPublisher) -> Self {
        Self { node_id, events_tx }
    }
}

#[async_trait]
impl dyswarm::server::Handler<NetworkEvent> for DyswarmHandler {
    async fn handle(&self, msg: DyswarmMessage<NetworkEvent>) -> dyswarm::types::Result<()> {
        match msg.data {
            NetworkEvent::PeerJoined {
                node_id,
                node_type,
                kademlia_peer_id,
                udp_gossip_addr,
                raptorq_gossip_addr,
                kademlia_liveness_addr,
                validator_public_key,
            } => {
                telemetry::info!("Node {} joined network", node_id);

                let evt = Event::PeerJoined(PeerData {
                    node_id,
                    node_type,
                    kademlia_peer_id,
                    udp_gossip_addr,
                    raptorq_gossip_addr,
                    kademlia_liveness_addr,
                    validator_public_key,
                });

                // TODO: once all known peers have been joined, send a `NetworkReady` event so a
                // dkg can be started and the first quorums can be formed

                let em = EventMessage::new(Some("network-events".into()), evt);

                self.events_tx.send(em).await.map_err(NodeError::from)?;
            },
            NetworkEvent::ClaimCreated { node_id, claim } => {
                telemetry::info!(
                    "Node ID {} recieved claim from {}: {}",
                    self.node_id,
                    node_id,
                    claim.public_key
                );

                let evt = Event::ClaimReceived(claim);
                let em = EventMessage::new(Some("network-events".into()), evt);

                self.events_tx.send(em).await.map_err(NodeError::from)?;
            },

            NetworkEvent::AssignmentToQuorumCreated {
                assigned_membership,
            } => {
                telemetry::info!(
                    "Node ID {} recieved assignment to quorum: {:?}",
                    self.node_id,
                    assigned_membership.quorum_kind
                );

                let evt = Event::QuorumMembershipAssigmentCreated(assigned_membership);
                let em = EventMessage::new(Some("consensus-events".into()), evt);

                self.events_tx.send(em).await.map_err(NodeError::from)?;
            },
            NetworkEvent::PartCommitmentCreated(node_id, part) => {
                let evt = Event::PartCommitmentCreated(node_id, part);
                let em = EventMessage::new(Some("consensus-events".into()), evt);

                self.events_tx.send(em).await.map_err(NodeError::from)?;
            },

            NetworkEvent::PartCommitmentAcknowledged { node_id, sender_id } => {
                let evt = Event::PartCommitmentAcknowledged { node_id, sender_id };
                let em = EventMessage::new(Some("consensus-events".into()), evt);
                self.events_tx.send(em).await.map_err(NodeError::from)?;
            },

            _ => {},
        }

        Ok(())
    }
}
