use crate::{components::network::NetworkEvent, NodeError};
use async_trait::async_trait;
use dyswarm::types::Message as DyswarmMessage;
use events::{Event, EventPublisher};
use primitives::NodeId;
use telemetry::error;

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
            } => {
                telemetry::info!("Node {} joined network", node_id);
            },
            NetworkEvent::ClaimCreated { node_id, claim } => {
                telemetry::info!(
                    "Node ID {} recieved claim from {}: {}",
                    self.node_id,
                    node_id,
                    claim.public_key
                );

                let evt = Event::ClaimReceived(claim);

                self.events_tx
                    .send(evt.into())
                    .await
                    .map_err(NodeError::from)?;
            },
            NetworkEvent::PartMessage(sender_id, part_committment) => {
                if let Err(err) = self
                    .events_tx
                    .send(Event::PartMessage(sender_id, part_committment).into())
                    .await
                {
                    error!(
                        "Error occurred while sending event to dkg module: {:?}",
                        err
                    );
                }
            },
            NetworkEvent::Ack(current_node_id, sender_id, ack_bytes) => {
                if let Err(err) = self
                    .events_tx
                    .send(Event::Ack(current_node_id, sender_id, ack_bytes).into())
                    .await
                {
                    error!(
                        "Error occurred while sending event to dkg module: {:?}",
                        err
                    );
                }
            },

            _ => {},
        }

        Ok(())
    }
}
