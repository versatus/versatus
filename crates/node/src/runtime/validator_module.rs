use std::path::PathBuf;

use crate::{result::Result, NodeError, RuntimeModule, RuntimeModuleState};
use state::{state::NetworkState, NodeState};
use tokio::sync::mpsc::error::TryRecvError;
use validator::validator_unit::ValidatorUnit;
use vrrb_core::event_router::{Event, Topic};

pub struct ValidatorModule {
    running_status: RuntimeModuleState,
}

/// ValidatorModule manages all validation tasks within VrrbNodes
/// it runs as an indepdendant module such that it can be enabled and disabled as necessary.
impl ValidatorModule {
    pub fn new() -> Self {
        Self {
            // validator: ValidatorUnit::new(),
            running_status: RuntimeModuleState::Stopped,
        }
    }
}
