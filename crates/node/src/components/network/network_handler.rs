use async_trait::async_trait;
use dyswarm::{server::ServerConfig, types::Message as DyswarmMessage};
use events::{Event, EventPublisher, EventSubscriber};
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
        // TODO: remove all that experimental hacky code below and replace it with
        // proper event publishing
        // TODO: deserialize network events into internal events and publish them so the
        // internal router can pick them up and forward them to the appropriate
        // components

        match msg.data {
            // TODO: handle all network events here
            NetworkEvent::Ping(node_id) => {
                let log_data = format!("ping from {} to {}", node_id, self.node_id);
                telemetry::info!("{}", log_data);
                println!("{}", node_id);
            },
            _ => {},
        }

        Ok(())
    }
}
