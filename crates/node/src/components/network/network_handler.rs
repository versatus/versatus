use std::net::SocketAddr;

use async_trait::async_trait;
use dyswarm::{server::ServerConfig, types::Message as DyswarmMessage};
use events::{Event, EventMessage, EventPublisher, EventSubscriber};
use primitives::NodeId;
use telemetry::info;

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

        println!("{} received: {:?}", self.node_id, msg);
        if &self.node_id == "node-0" {
            self.events_tx
                .send(Event::Other("Hello world".to_string()).into())
                .await
                .unwrap();
        }

        Ok(())
    }
}
