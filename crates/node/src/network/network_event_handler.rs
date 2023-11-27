use async_trait::async_trait;
use dyswarm::types::Message as DyswarmMessage;
use events::{Event, EventMessage, EventPublisher, PeerData, Topic};
use primitives::{NodeId, NETWORK_TOPIC_STR, RUNTIME_TOPIC_STR};
use vrrb_core::transactions::Transaction;

use crate::{network::NetworkEvent, NodeError, Result};

#[derive(Debug, Clone)]
pub struct DyswarmHandler {
    pub node_id: NodeId,
    pub events_tx: EventPublisher,
}

impl DyswarmHandler {
    pub fn new(node_id: NodeId, events_tx: EventPublisher) -> Self {
        Self { node_id, events_tx }
    }

    async fn send_event(&self, topic: &str, evt: Event) -> Result<()> {
        let em = EventMessage::new(Some(topic.into()), evt);
        self.events_tx.send(em).await.map_err(NodeError::from)
    }

    pub async fn send_event_to_network(&self, evt: Event) -> Result<()> {
        self.send_event(NETWORK_TOPIC_STR, evt).await
    }

    pub async fn send_event_to_runtime(&self, evt: Event) -> Result<()> {
        self.send_event(RUNTIME_TOPIC_STR, evt).await
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

                if let Err(err) = self.send_event_to_network(evt).await {
                    telemetry::error!("{}", err);
                }
            }
            NetworkEvent::ClaimCreated { node_id, claim } => {
                telemetry::info!(
                    "Node ID {} recieved claim from {}: {}",
                    self.node_id,
                    node_id,
                    claim.public_key
                );

                let evt = Event::ClaimReceived(claim);
                if let Err(err) = self.send_event_to_network(evt).await {
                    telemetry::error!("{}", err);
                }
            }

            NetworkEvent::QuorumMembershipAssigmentsCreated(assignments) => {
                telemetry::info!(
                    "Node ID {} recieved {} assignments",
                    self.node_id,
                    assignments.len(),
                );

                let evt = Event::QuorumMembershipAssigmentsCreated(assignments);
                if let Err(err) = self.send_event_to_runtime(evt).await {
                    telemetry::error!("{}", err);
                }
            }
            NetworkEvent::AssignmentToQuorumCreated {
                assigned_membership,
            } => {
                telemetry::info!(
                    "Node ID {} recieved assignment to quorum: {:?}",
                    self.node_id,
                    assigned_membership.quorum_kind
                );

                let evt = Event::QuorumMembershipAssigmentCreated(assigned_membership);
                if let Err(err) = self.send_event_to_runtime(evt).await {
                    telemetry::error!("{}", err);
                }
            }
            NetworkEvent::PartCommitmentCreated(node_id, part) => {
                let evt = Event::PartCommitmentCreated(node_id, part);
                if let Err(err) = self.send_event_to_runtime(evt).await {
                    telemetry::error!("{}", err);
                }
            }

            NetworkEvent::PartCommitmentAcknowledged {
                node_id,
                sender_id,
                ack,
            } => {
                let evt = Event::PartCommitmentAcknowledged {
                    node_id,
                    sender_id,
                    ack,
                };

                if let Err(err) = self.send_event_to_runtime(evt).await {
                    telemetry::error!("{}", err);
                }
            }
            NetworkEvent::NewTxnForwarded(node_id, txn) => {
                let evt = Event::NewTxnForwarded(node_id, txn);
                if let Err(err) = self.send_event_to_runtime(evt).await {
                    telemetry::error!("{}", err);
                }
            }
            NetworkEvent::TransactionVoteCreated(vote) => {
                let evt = Event::TransactionVoteCreated(vote);
                if let Err(err) = self.send_event_to_runtime(evt).await {
                    telemetry::error!("{}", err);
                }
            }
            NetworkEvent::BlockCreated(block) => {
                let evt = Event::BlockCreated(block);
                if let Err(err) = self.send_event_to_runtime(evt).await {
                    telemetry::error!("{}", err);
                }
            }

            _ => {}
        }

        Ok(())
    }
}
