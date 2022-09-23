use commands::command::Command;
use tokio::sync::mpsc::UnboundedReceiver;

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
pub trait RuntimeModule {
    fn name(&self) -> String;
    fn status(&self) -> RuntimeModuleState;
    fn start(&mut self, control_rx: &mut UnboundedReceiver<Command>) -> Result<()>;
}
