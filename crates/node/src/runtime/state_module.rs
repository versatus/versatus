use std::{hash::Hash, path::PathBuf};

use async_trait::async_trait;
use lr_trie::ReadHandleFactory;
use patriecia::{db::MemoryDB, inner::InnerTrie};
use state::{NodeState, NodeStateConfig, NodeStateReadHandle};
use telemetry::info;
use theater::{Actor, ActorId, ActorLabel, ActorState, Handler, Message, TheaterError};
use tokio::sync::broadcast::error::TryRecvError;
use vrrb_core::event_router::{DirectedEvent, Event, Topic};

use crate::{result::Result, NodeError, RuntimeModule};

pub struct StateModuleConfig {
    pub node_state: NodeState,
    pub events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
}

#[derive(Debug)]
pub struct StateModule {
    state: NodeState,
    status: ActorState,
    label: ActorLabel,
    id: ActorId,
    events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
}

/// StateModule manages all state persistence and updates within VrrbNodes
/// it runs as an indepdendant module such that it can be enabled and disabled
/// as necessary.
impl StateModule {
    pub fn new(config: StateModuleConfig) -> Self {
        Self {
            state: config.node_state,
            events_tx: config.events_tx,
            status: ActorState::Stopped,
            label: String::from("State"),
            id: uuid::Uuid::new_v4().to_string(),
        }
    }
}

impl StateModule {
    fn name(&self) -> String {
        String::from("State module")
    }

    /// Produces a reader factory that can be used to generate read handles into
    /// the state tree.
    #[deprecated(note = "use self.read_handle instead")]
    pub fn factory(&self) -> ReadHandleFactory<InnerTrie<MemoryDB>> {
        self.state.factory()
    }

    pub fn read_handle(&self) -> NodeStateReadHandle {
        self.state.read_handle()
    }
}

#[async_trait]
impl Handler<Event> for StateModule {
    fn id(&self) -> ActorId {
        self.id.clone()
    }

    fn label(&self) -> ActorLabel {
        self.name()
    }

    fn status(&self) -> ActorState {
        self.status.clone()
    }

    fn set_status(&mut self, actor_status: ActorState) {
        self.status = actor_status;
    }

    fn on_stop(&self) {
        info!(
            "{}-{} received stop signal. Stopping",
            self.name(),
            self.label()
        );
    }

    async fn handle(&mut self, event: Event) -> theater::Result<ActorState> {
        match event {
            Event::Stop => {
                // TODO: fix
                // self.state
                //     .serialize_to_json()
                //     .map_err(|err| NodeError::Other(err.to_string()))?;
                return Ok(ActorState::Stopped);
            },

            Event::NewTxnCreated(txn) => {
                info!("Storing transaction in mempool for validation");

                let txn_hash = txn.digest_bytes();

                self.state.add_txn_to_mempool(txn);

                self.events_tx
                    .send((Topic::Transactions, Event::TxnAddedToMempool(txn_hash)))
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },
            Event::NoOp => {},
            _ => telemetry::warn!("Unrecognized command received: {:?}", event),
        }

        Ok(ActorState::Running)
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use theater::ActorImpl;
    use vrrb_core::{
        event_router::{DirectedEvent, Event},
        txn::NULL_TXN,
    };

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

        let mut state_module = ActorImpl::new(state_module);

        let (ctrl_tx, mut ctrl_rx) = tokio::sync::broadcast::channel::<Event>(10);

        assert_eq!(state_module.status(), ActorState::Stopped);

        let handle = tokio::spawn(async move {
            state_module.start(&mut ctrl_rx).await.unwrap();
            assert_eq!(state_module.status(), ActorState::Terminating);
        });

        ctrl_tx.send(Event::Stop.into()).unwrap();

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

        let mut state_module = ActorImpl::new(state_module);

        let (ctrl_tx, mut ctrl_rx) = tokio::sync::broadcast::channel::<Event>(10);

        assert_eq!(state_module.status(), ActorState::Stopped);

        let handle = tokio::spawn(async move {
            state_module.start(&mut ctrl_rx).await.unwrap();
        });

        ctrl_tx.send(Event::NewTxnCreated(NULL_TXN)).unwrap();
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

        let mut state_module = ActorImpl::new(state_module);

        let events_handle = tokio::spawn(async move {
            events_rx.recv().await.unwrap();
        });

        let (ctrl_tx, mut ctrl_rx) = tokio::sync::broadcast::channel::<Event>(10);

        assert_eq!(state_module.status(), ActorState::Stopped);

        let handle = tokio::spawn(async move {
            state_module.start(&mut ctrl_rx).await.unwrap();
        });

        // TODO: implement all state && validation ops

        ctrl_tx.send(Event::NewTxnCreated(NULL_TXN)).unwrap();
        ctrl_tx.send(Event::Stop).unwrap();

        handle.await.unwrap();
        events_handle.await.unwrap();
    }
}
