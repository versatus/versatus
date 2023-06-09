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

pub type RuntimeHandle = Option<JoinHandle<Result<()>>>;
pub type RaptorHandle = Option<thread::JoinHandle<bool>>;
pub type SchedulerHandle = Option<thread::JoinHandle<()>>;

#[derive(Debug)]
pub struct RuntimeComponentHandle<D: Sized> {
    task_handle: RuntimeHandle,
    data: D,
}

impl<D: Sized + Clone> RuntimeComponentHandle<D> {
    pub fn new(task_handle: RuntimeHandle, data: D) -> Self {
        Self { task_handle, data }
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
    pub mempool_handle: RuntimeHandle,
    pub state_handle: RuntimeHandle,
    pub gossip_handle: RuntimeHandle,
    pub jsonrpc_server_handle: RuntimeHandle,
    pub miner_handle: RuntimeHandle,
    pub dkg_handle: RuntimeHandle,
    pub miner_election_handle: RuntimeHandle,
    pub quorum_election_handle: RuntimeHandle,
    pub farmer_handle: RuntimeHandle,
    pub harvester_handle: RuntimeHandle,
    pub indexer_handle: RuntimeHandle,
    pub dag_handle: RuntimeHandle,
    pub raptor_handle: RaptorHandle,
    pub scheduler_handle: SchedulerHandle,
    pub node_gui_handle: RuntimeHandle,
}
