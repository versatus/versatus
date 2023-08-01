use std::{collections::HashMap, thread};

use tokio::task::JoinHandle;

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

pub type RuntimeComponentLabel = String;
pub type RuntimeHandle = JoinHandle<Result<()>>;
pub type OptionalRuntimeHandle = Option<(RuntimeHandle, RuntimeComponentLabel)>;
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

    pub fn label(&self) -> String {
        self.label.clone()
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

#[derive(Debug, Default)]
pub struct RuntimeComponentManager {
    components: HashMap<RuntimeComponentLabel, RuntimeHandle>,
}

impl RuntimeComponentManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a RuntimeComponentHandle within the manager's store.
    pub fn register_component(&mut self, label: RuntimeComponentLabel, handle: RuntimeHandle) {
        self.components.insert(label, handle);
    }

    pub async fn stop(self) -> crate::Result<()> {
        for (label, handle) in self.components {
            handle.await??;
            telemetry::info!("Shutdown complete for {label}");
        }

        Ok(())
    }
}
