
use async_trait::async_trait;
use state::{state::NetworkState, NodeState};
use tokio::sync::broadcast::error::TryRecvError;
use vrrb_core::event_router::{DirectedEvent, Event, Topic};

use crate::{result::Result, NodeError, RuntimeModule, RuntimeModuleState};

pub struct BroadcastModuleConfig {
    pub path: PathBuf,
    pub events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
}

pub struct BroadcastModule {
    state: NodeState,
    running_status: RuntimeModuleState,
    events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
}

/// StateModule manages all state persistence and updates within VrrbNodes
/// it runs as an indepdendant module such that it can be enabled and disabled
/// as necessary.
impl BoradcastModule {
    pub fn new(config: StateModuleConfig) -> Self {
        Self {
            state: NodeState::new(config.path),
            running_status: RuntimeModuleState::Stopped,
            events_tx: config.events_tx,
        }
    }
}
