use qp2p::{ConnectionError, EndpointError, SendError};
use serde::{Deserialize, Serialize};
use theater::TheaterError;
use thiserror::Error;
use udp2p::node::peer_id::PeerId;

/// `Topology` is a struct that contains the number of master nodes, the number
/// of quorum nodes, and the miner network id.
///
/// Properties:
///
/// * `num_of_master_nodes`: The number of master nodes in the network.
/// * `num_of_quorum_nodes`: The number of nodes in the quorum.
/// * `miner_network_id`: The peer id of the miner node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Eq)]
pub struct Topology {
    pub num_of_master_nodes: usize,
    pub num_of_quorum_nodes: usize,
    pub miner_network_id: PeerId,
}

impl Topology {
    pub fn new(
        num_of_master_nodes: usize,
        num_of_quorum_nodes: usize,
        miner_network_id: PeerId,
    ) -> Self {
        Self {
            num_of_master_nodes,
            num_of_quorum_nodes,
            miner_network_id,
        }
    }
}

#[derive(Debug)]
pub enum BroadcastStatus {
    ConnectionEstablished,
    Success,
}

/// List of all possible errors related to BroadCasting .
#[derive(Error, Debug)]
pub enum BroadcastError {
    #[error("Connection error: {0}")]
    Connection(#[from] ConnectionError),

    #[error("Send error: {0}")]
    Send(#[from] SendError),

    #[error("Endpoint error: {0}")]
    Endpoint(#[from] EndpointError),

    #[error("Udp Port already in use")]
    EaddrInUse,

    #[error("Current Node doesn't have any peers")]
    NoPeers,

    #[error("error: {0}")]
    Other(String),
}

#[deprecated(note = "here for backwards compatibility")]
pub type BroadCastError = BroadcastError;

impl From<BroadcastError> for TheaterError {
    fn from(err: BroadcastError) -> Self {
        telemetry::error!("Broadcast error: {err}");
        TheaterError::Other(err.to_string())
    }
}
