use std::fmt::Display;

#[deprecated(note = "Use NodeState directly instead")]
pub type RuntimeModuleState = NodeState;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum NodeState {
    Starting,
    Running,
    Stopped,
    Terminating,
}

impl Display for NodeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = match self {
            NodeState::Starting => "Starting",
            NodeState::Running => "Running",
            NodeState::Stopped => "Stopped",
            NodeState::Terminating => "Terminating",
        };

        write!(f, "{}", state)
    }
}
