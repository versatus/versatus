use serde::{Deserialize, Serialize};
use qp2p::{ConnectionError, EndpointError, SendError};
use udp2p::node::peer_id::PeerId;
use thiserror::Error;

/// `Topology` is a struct that contains the number of regular nodes, master nodes, quorum, peers, and
/// master node quorum peers.
///
/// Properties:
///
/// * `num_of_regular_nodes`: The number of regular nodes in the network.
/// * `num_of_master_nodes`: The number of master nodes in the network.
/// * `num_of_quorum`: The number of nodes that are required to form a quorum.
/// * `master_node_quorum_peers`: This is a HashMap of PeerId and PeerInfo. The PeerId is the public key
/// of the master node and the PeerInfo is the information about the master node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Topology {
    //pub total_nodes: usize, **not queriable yet; could be added to state
    pub num_of_master_nodes: usize,
    pub num_of_quorum_nodes: usize,
    // pub quorum: Quorum,
    pub miner_network_id: PeerId,
}

impl Topology {
    pub fn new(
        //  quorum: &Quorum,
        num_of_master_nodes:usize,
        num_of_quorum_nodes:usize,
        miner_network_id: PeerId,
    ) -> Self {
        Self {
            //total_nodes,
            num_of_master_nodes,
            num_of_quorum_nodes,
            miner_network_id,
        }
    }
}


#[derive(Debug)]
pub enum BroadCastResult {
   ConnectionEstablished,
   Success
}


/// List of all possible errors related to BroadCasting .
#[derive(Error, Debug)]
pub enum BroadCastError {
    #[error("There was a problem while creating endpoint")]
    EndpointError(#[from] EndpointError),
    #[error("There was a problem while connecting to endpoint")]
    ConnectionError(#[from] ConnectionError),
    #[error("There was a problem while broadcasting data to peers")]
    BroadcastingDataError(#[from] SendError),

}
