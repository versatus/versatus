use std::thread;

use tokio::task::JoinHandle;
use vrrb_config::NodeConfig;

use crate::Result;

#[derive(Debug, Clone)]
pub struct RuntimeComponentHealthReport {}

#[async_trait::async_trait]
/// Represents a component that can be installed into a node's runtime. These
/// can be enabled, disabled and uninstalled and are meant to provide fine
/// grained control over a node's behavior.
pub trait RuntimeComponent<A, D>
where
    A: Sized,
    D: Sized,
{
    async fn setup(args: A) -> Result<RuntimeComponentHandle<D>>;
    async fn stop(&mut self) -> Result<()>;
}

pub type RuntimeHandle = JoinHandle<Result<()>>;
pub type OptionalRuntimeHandle = Option<RuntimeHandle>;
pub type RaptorHandle = Option<thread::JoinHandle<bool>>;
pub type SchedulerHandle = Option<thread::JoinHandle<()>>;

#[derive(Debug)]
pub struct RuntimeComponentHandle<D: Sized> {
    label: String,
    task_handle: RuntimeHandle,
    data: D,
}

impl<D: Sized + Clone> RuntimeComponentHandle<D> {
    pub fn new(task_handle: RuntimeHandle, data: D, label: String) -> Self {
        Self {
            task_handle,
            data,
            label,
        }
    }

    pub fn handle(self) -> RuntimeHandle {
        self.task_handle
    }

    pub fn data(&self) -> D {
        self.data.clone()
    }

    pub fn data_ref(&self) -> &D {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut D {
        &mut self.data
    }
}

#[derive(Debug)]
pub struct RuntimeComponents {
    pub node_config: NodeConfig,
    pub mempool_handle: OptionalRuntimeHandle,
    pub state_handle: OptionalRuntimeHandle,
    pub gossip_handle: OptionalRuntimeHandle,
    pub jsonrpc_server_handle: OptionalRuntimeHandle,
    pub miner_handle: OptionalRuntimeHandle,
    pub dkg_handle: OptionalRuntimeHandle,
    pub miner_election_handle: OptionalRuntimeHandle,
    pub quorum_election_handle: OptionalRuntimeHandle,
    pub farmer_handle: OptionalRuntimeHandle,
    pub harvester_handle: OptionalRuntimeHandle,
    pub indexer_handle: OptionalRuntimeHandle,
    pub dag_handle: OptionalRuntimeHandle,
    pub raptor_handle: RaptorHandle,
    pub scheduler_handle: SchedulerHandle,
    pub node_gui_handle: OptionalRuntimeHandle,
}
