use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use udp2p::node::PeerId;



/// `Topology` is a struct that contains the number of regular nodes, master nodes, quorum, peers, and
/// master node quorum peers.
///
/// Properties:
///
/// * `num_of_regular_nodes`: The number of regular nodes in the network.
/// * `num_of_master_nodes`: The number of master nodes in the network.
/// * `num_of_quorum`: The number of nodes that are required to form a quorum.
/// * `peers`: This is a map of all the peers in the network. The key is the peer's ID and the value is
/// the peer's information.
/// * `master_node_quorum_peers`: This is a HashMap of PeerId and PeerInfo. The PeerId is the public key
/// of the master node and the PeerInfo is the information about the master node.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Topology {
    pub total_nodes: usize,
    pub num_of_master_nodes: usize,
    pub quorum_size:usize,
    pub peers:HashMap<PeerId,PeerInfo>,
    pub quorum: &Quorum,
    pub miner_info: PeerId
    
}




//Notes

/* 
ToDo

pub enum Broadcast_type{
    QuicBroadCast
    UdpBroadcast
}


pub enum ConnectionType{
    Regular---SendingBatch/Gossip
    MinerToMasterNode--- ProposingTheBlock
    MasterToMaster--DKG/BlockVerification
    MasterToRegular--ChainLock
}
#[derive(Debug, Clone)]
pub struct BroadcastEngine {
    // mutex of node state
    // holds node's state mutable ref
    network_topology: Arc<RwLock<Topology>>,
    current_node: Arc<RwLock<Node>>, 
    // connection_list
    pub connection_list: Arc<Mutex<QuicConnectionList>>,   ()
    
    pub connection_manager_in: Channel<ConnectionMessage>, 

  
}


impl{


    new(){

    }


    broadcasting_data_via_quic(br)

    broadcasting_data_via_raptor(br)


    send_data_via_quic(data,peerId)
        



}

run(

     connection_sender--
                                 reciever ---- Match (Command )


     message_sender---

    reciever 
    AddPeers(Vec<PeerIds>)

    Data(DataType::Vec::<u8>) 
)

*/
