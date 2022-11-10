use async_trait::async_trait;
use tokio::sync::mpsc::UnboundedReceiver;
use vrrb_core::event_router::Event;

use crate::result::Result;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RuntimeModuleState {
    Starting,
    Running,
    Stopped,
    Terminating,
}

/// RuntimeModule represents a node component that is loaded on startup and
/// controls whenever a node is terminated
#[async_trait]
pub trait RuntimeModule {
    fn name(&self) -> String;
    fn status(&self) -> RuntimeModuleState;
    async fn start(&mut self, control_rx: &mut UnboundedReceiver<Event>) -> Result<()>;
}
