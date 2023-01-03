use std::path::PathBuf;

use async_trait::async_trait;
use lr_trie::ReadHandleFactory;
use patriecia::{db::MemoryDB, inner::InnerTrie};
use state::{NodeState, NodeStateConfig, NodeStateReadHandle};
use telemetry::info;
use tokio::sync::broadcast::error::TryRecvError;
use vrrb_core::event_router::{DirectedEvent, Event, Topic};

use crate::{result::Result, NodeError, RuntimeModule, RuntimeModuleState};

pub struct StateModuleConfig {
    pub node_state: NodeState,
    pub events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
}

#[derive(Debug)]
pub struct StateModule {
    state: NodeState,
    running_status: RuntimeModuleState,
    events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
}

/// StateModule manages all state persistence and updates within VrrbNodes
/// it runs as an indepdendant module such that it can be enabled and disabled
/// as necessary.
impl StateModule {
    pub fn new(config: StateModuleConfig) -> Self {
        Self {
            state: config.node_state,
            running_status: RuntimeModuleState::Stopped,
            events_tx: config.events_tx,
        }
    }
}

impl StateModule {
    /// Produces a reader factory that can be used to generate read handles into
    /// the state tree.
    #[deprecated(note = "use self.read_handle instead")]
    pub fn factory(&self) -> ReadHandleFactory<InnerTrie<MemoryDB>> {
        self.state.factory()
    }

    pub fn read_handle(&self) -> NodeStateReadHandle {
        self.state.read_handle()
    }

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
                info!("Storing transaction in tx tree.");
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
impl RuntimeModule for StateModule {
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
        info!("{0} started", self.name());

        while let Ok(event) = events_rx.recv().await {
            info!("{} received {event:?}", self.name());

            if event == Event::Stop {
                info!("{0} received stop signal. Stopping", self.name());

                self.running_status = RuntimeModuleState::Terminating;

                // TODO: fix
                // self.state
                //     .serialize_to_json()
                //     .map_err(|err| NodeError::Other(err.to_string()))?;

                break;
            }

            self.process_event(event);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use vrrb_core::event_router::{DirectedEvent, Event};

    use super::*;

    #[tokio::test]
    async fn state_runtime_module_starts_and_stops() {
        let temp_dir_path = env::temp_dir().join("state.json");

        let (events_tx, _) = tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();

        let node_state_config = NodeStateConfig {
            path: temp_dir_path,
            serialized_state_filename: None,
            serialized_mempool_filename: None,
            serialized_confirmed_txns_filename: None,
        };

        let node_state = NodeState::new(&node_state_config);

        let mut state_module = StateModule::new(StateModuleConfig {
            events_tx,
            node_state,
        });

        let (ctrl_tx, mut ctrl_rx) = tokio::sync::broadcast::channel::<Event>(10);

        assert_eq!(state_module.status(), RuntimeModuleState::Stopped);

        let handle = tokio::spawn(async move {
            state_module.start(&mut ctrl_rx).await.unwrap();
            assert_eq!(state_module.status(), RuntimeModuleState::Stopped);
        });

        ctrl_tx.send(Event::Stop).unwrap();

        handle.await.unwrap();
    }

    #[tokio::test]
    async fn state_runtime_receives_new_txn_event() {
        let temp_dir_path = env::temp_dir().join("state.json");

        let (events_tx, _) = tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();

        let node_state_config = NodeStateConfig {
            path: temp_dir_path,
            serialized_state_filename: None,
            serialized_mempool_filename: None,
            serialized_confirmed_txns_filename: None,
        };

        let node_state = NodeState::new(&node_state_config);

        let mut state_module = StateModule::new(StateModuleConfig {
            events_tx,
            node_state,
        });

        let (ctrl_tx, mut ctrl_rx) = tokio::sync::broadcast::channel::<Event>(1);

        assert_eq!(state_module.status(), RuntimeModuleState::Stopped);

        let handle = tokio::spawn(async move {
            state_module.start(&mut ctrl_rx).await.unwrap();
            assert_eq!(state_module.status(), RuntimeModuleState::Stopped);
        });

        ctrl_tx.send(Event::TxnCreated(vec![])).unwrap();
        ctrl_tx.send(Event::Stop).unwrap();

        handle.await.unwrap();
    }

    #[tokio::test]
    async fn state_runtime_can_publish_events() {
        let temp_dir_path = env::temp_dir().join("state.json");

        let (events_tx, mut events_rx) = tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();

        let node_state_config = NodeStateConfig {
            path: temp_dir_path,
            serialized_state_filename: None,
            serialized_mempool_filename: None,
            serialized_confirmed_txns_filename: None,
        };

        let node_state = NodeState::new(&node_state_config);

        let mut state_module = StateModule::new(StateModuleConfig {
            events_tx,
            node_state,
        });

        let events_handle = tokio::spawn(async move {
            events_rx.recv().await.unwrap();
        });

        let (ctrl_tx, mut ctrl_rx) = tokio::sync::broadcast::channel::<Event>(10);

        assert_eq!(state_module.status(), RuntimeModuleState::Stopped);

        let handle = tokio::spawn(async move {
            state_module.start(&mut ctrl_rx).await.unwrap();
            assert_eq!(state_module.status(), RuntimeModuleState::Stopped);
        });

        ctrl_tx.send(Event::TxnCreated(vec![])).unwrap();
        ctrl_tx.send(Event::Stop).unwrap();

        handle.await.unwrap();
        events_handle.await.unwrap();
    }
}
