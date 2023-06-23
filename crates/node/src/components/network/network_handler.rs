use async_trait::async_trait;
use dyswarm::types::Message as DyswarmMessage;
use events::EventPublisher;
use primitives::NodeId;

use crate::components::network::NetworkEvent;

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
                // telemetry::info!("Node {} joined network", node_id);
                println!("{} node {} joined network", node_type, node_id);
            },
            NetworkEvent::Broadcast(claim)
             => {
                // telemetry::info!("Node {} joined network", node_id);
                println!("Node ID {:?} recieved claim {:#?}", claim.public_key,self.node_id);
            },

            _ => {},
        }

        Ok(())
    }
}
