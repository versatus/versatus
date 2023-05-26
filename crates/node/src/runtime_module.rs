use async_trait::async_trait;
use events::Event;
use tokio::sync::broadcast::Receiver;

use crate::result::Result;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RuntimeModuleState {
    Starting,
    Running,
    Stopped,
    Terminating,
}
