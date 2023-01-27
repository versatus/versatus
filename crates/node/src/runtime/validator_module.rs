use std::{path::PathBuf, result::Result as StdResult};

use async_trait::async_trait;
use telemetry::info;
use tokio::sync::broadcast::{error::TryRecvError, Receiver};
use vrrb_core::event_router::{Event, Topic};

use crate::{result::Result, NodeError, RuntimeModule, RuntimeModuleState};

pub struct ValidatorModule {
    running_status: RuntimeModuleState,
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
}

#[async_trait]
impl RuntimeModule for ValidatorModule {
    fn name(&self) -> String {
        String::from("Validator module")
    }

    fn status(&self) -> RuntimeModuleState {
        self.running_status.clone()
    }

    async fn start(&mut self, events_rx: &mut Receiver<Event>) -> Result<()> {
        info!("{0} started", self.name());

        while let Ok(event) = events_rx.recv().await {
            info!("{} received {event:?}", self.name());

            if event == Event::Stop {
                info!("{0} received stop signal. Stopping", self.name());

                self.running_status = RuntimeModuleState::Terminating;

                break;
            }

            self.process_event(event);
        }

        self.running_status = RuntimeModuleState::Stopped;

        Ok(())
    }
}

impl ValidatorModule {
    fn decode_event(&mut self, event: StdResult<Event, TryRecvError>) -> Event {
        match event {
            Ok(cmd) => cmd,
            Err(err) => match err {
                TryRecvError::Closed => {
                    telemetry::error!("the events channel has been closed.");
                    Event::Stop
                },

                TryRecvError::Lagged(u64) => {
                    telemetry::error!("receiver lagged behind");
                    Event::NoOp
                },
                _ => Event::NoOp,
            },
            _ => Event::NoOp,
        }
    }

    fn process_event(&mut self, event: Event) {
        match event {
            Event::BlockConfirmed(_) => {
                // do something
            },
            Event::NoOp => {},
            _ => telemetry::warn!("unrecognized command received: {:?}", event),
        }
    }
}
