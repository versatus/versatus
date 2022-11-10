use thiserror::Error;
use std::fmt;

#[derive(Debug, Error)]
pub enum DiscovererError {

    #[error("Failed to deserialize NodeRouteEntry from bytes: `{0}`")]
    Deserialize(String),
}

#[derive(PartialEq, Eq, Debug)]
pub enum NodePoolError {
}

// TODO: fix compiler
#[derive(PartialEq, Eq, Debug)]
#[allow(dead_code)]
pub enum BootstrapError {
    NodeMissing,
    NodeOutOfSync,
    GeneralConnectionError,
}

impl fmt::Display for BootstrapError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &*self {
            BootstrapError::NodeMissing => write!(f, "Connected node went missing during synchronization"),
            BootstrapError::NodeOutOfSync => write!(f, "This node is out of sync as of now"),
            BootstrapError::GeneralConnectionError => write!(f, "Connection Error - network problems."),
        }
    }
}

// TODO: fix compiler.
#[allow(dead_code)]
#[derive(PartialEq, Eq, Debug)]
pub enum DataBrokerError {
    ConnectionError,
    TransmissionError,
}
