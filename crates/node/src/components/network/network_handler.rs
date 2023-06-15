use std::net::SocketAddr;

use async_trait::async_trait;
use dyswarm::{server::ServerConfig, types::Message as DyswarmMessage};
use events::{Event, EventMessage, EventPublisher, EventSubscriber};
use telemetry::info;

use crate::components::network::NetworkEvent;

#[derive(Debug, Clone)]
pub struct DyswarmHandler {
    pub events_tx: EventPublisher,
}

impl DyswarmHandler {
    pub fn new(events_tx: EventPublisher) -> Self {
        Self { events_tx }
    }
}

#[async_trait]
impl dyswarm::server::Handler<NetworkEvent> for DyswarmHandler {
    async fn handle(&self, msg: DyswarmMessage<NetworkEvent>) -> dyswarm::types::Result<()> {
        // TODO: deserialize network events into internal events and publish them so the
        // internal router can pick them up and forward them to the appropriate
        // components
        //
        Ok(())
    }
}
