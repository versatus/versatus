#[deprecated(note = "Use NodeState directly instead")]
pub type RuntimeModuleState = NodeState;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum NodeState {
    Starting,
    Running,
    Stopped,
    Terminating,
}
