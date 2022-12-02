use async_trait::async_trait;
use state::{state::NetworkState, NodeState};
use tokio::sync::broadcast::error::TryRecvError;
use vrrb_core::event_router::{DirectedEvent, Event, Topic};

use crate::{result::Result, NodeError, RuntimeModule, RuntimeModuleState};

pub struct BroadcastModule {
    running_status: RuntimeModuleState,
    msgs: tokio::sync::mpsc::UnboundedSender<Message>,
    msg_handler: tokio::sync::mpsc::UnboundedReceiver<Message>,
}

/// StateModule manages all state persistence and updates within VrrbNodes
/// it runs as an indepdendant module such that it can be enabled and disabled
/// as necessary.
impl BroadcastModule {
    pub fn new() -> Self {
        Self {
            running_status: RuntimeModuleState::Stopped,
            msgs: config.events_tx,
        }
    }
}

impl BroadcastModule {
    fn decode_event(&mut self, event: std::result::Result<Event, TryRecvError>) -> Event {
        match event {
            Ok(cmd) => cmd,
            Err(err) => match err {
                TryRecvError::Closed => {
                    telemetry::error!("The events channel for event router has been closed.");
                    Event::Stop
                },

                TryRecvError::Lagged(u64) => {
                    telemetry::error!("Receiver lagged behind");
                    Event::NoOp
                },
                _ => Event::NoOp,
            },
            _ => Event::NoOp,
        }
    }

    fn process_event(&mut self, event: Event) {
        match event {
            Event::TxnCreated(_) => {
                telemetry::info!("Storing transaction in tx tree.");
                self.events_tx
                    .send((Topic::Transactions, Event::TxnCreated(vec![])))
                    .unwrap();
            },
            Event::NoOp => {},
            _ => telemetry::warn!("Unrecognized command received: {:?}", event),
        }
    }
}

#[async_trait]
impl RuntimeModule for BroadcastModule {
    fn name(&self) -> String {
        String::from("State module")
    }

    fn status(&self) -> RuntimeModuleState {
        self.running_status.clone()
    }

    async fn start(
        &mut self,
        events_rx: &mut tokio::sync::broadcast::Receiver<Event>,
    ) -> Result<()> {
        loop {
            let event = self.decode_event(events_rx.try_recv());

            if event == Event::Stop {
                telemetry::info!("{0} received stop signal. Stopping", self.name());

                self.running_status = RuntimeModuleState::Terminating;

                self.state
                    .serialize_to_json()
                    .map_err(|err| NodeError::Other(err.to_string()))?;

                break;
            }

            self.process_event(event);
        }

        self.running_status = RuntimeModuleState::Stopped;

        Ok(())
    }
}



