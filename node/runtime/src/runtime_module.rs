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
    fn start(&self) -> Result<()>;
    fn stop(&self) -> Result<()>;
    fn force_stop(&self);
}
