use std::{path::PathBuf, sync::Arc};

use async_trait::async_trait;
use lr_trie::{LeftRightTrie, ReadHandleFactory};
use patriecia::db::MemoryDB;
use state::{state::NetworkState, NodeState};
use tokio::sync::mpsc::{channel, error::TryRecvError, unbounded_channel};
use validator::validator_unit::ValidatorUnit;
use vrrb_core::event_router::{Event, Topic};

use crate::{result::Result, NodeError, RuntimeModule, RuntimeModuleState};

pub struct ValidatorModule {
    running_status: RuntimeModuleState,
    // TODO: enable once router loop bug is fixed
    // validator: ValidatorUnit<MemoryDB>,
}

/// ValidatorModule manages all validation tasks within VrrbNodes
/// it runs as an indepdendant module such that it can be enabled and disabled
/// as necessary.
impl ValidatorModule {
    pub fn new() -> Self {
        Self {
            running_status: RuntimeModuleState::Stopped,
        }
    }

    fn process_event(&mut self, event: Event) {
        match event {
            _ => telemetry::warn!("Unrecognized command received: {:?}", event),
        }
    }

    fn decode_event(&mut self, event: std::result::Result<Event, TryRecvError>) -> Event {
        match event {
            Ok(cmd) => cmd,
            Err(err) if err == TryRecvError::Disconnected => {
                telemetry::error!("The events channel for event router has been closed.");
                Event::Stop
            },
            _ => Event::NoOp,
        }
    }
}

#[async_trait]
impl RuntimeModule for ValidatorModule {
    fn name(&self) -> String {
        String::from("Validator module")
    }

    fn status(&self) -> RuntimeModuleState {
        self.running_status.clone()
    }

    async fn start(
        &mut self,
        event_stream: &mut tokio::sync::mpsc::UnboundedReceiver<Event>,
    ) -> Result<()> {
        loop {
            let event = self.decode_event(event_stream.try_recv());

            if event == Event::Stop {
                telemetry::info!("{0} received stop signal. Stopping", self.name());

                self.running_status = RuntimeModuleState::Terminating;

                break;
            }

            self.process_event(event);
        }

        self.running_status = RuntimeModuleState::Stopped;

        Ok(())
    }
}
