use std::net::SocketAddr;

use events::Vote;
use mempool::TxnRecord;
use primitives::{FarmerQuorumThreshold, NodeId, NodeType, PeerId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
/// Represents data trasmitted over the VRRB network by nodes that participate
/// in it
pub enum NetworkEvent {
    InvalidBlock {
        block_height: u128,
        reason: Vec<u8>,
        miner_id: String,
        sender_id: String,
    },
    Disconnect {
        sender_id: String,
        pubkey: String,
    },
    StateComponents {
        data: Vec<u8>,
        requestor: String,
        requestor_id: String,
        sender_id: String,
    },
    Genesis {
        data: Vec<u8>,
        requestor: String,
        requestor_id: String,
        sender_id: String,
    },
    Child {
        data: Vec<u8>,
        requestor: String,
        requestor_id: String,
        sender_id: String,
    },
    Parent {
        data: Vec<u8>,
        requestor: String,
        requestor_id: String,
        sender_id: String,
    },
    Ledger {
        data: Vec<u8>,
        requestor: String,
        requestor_id: String,
        sender_id: String,
    },
    NetworkState {
        data: Vec<u8>,
        requestor: String,
        requestor_id: String,
        sender_id: String,
    },
    ClaimAbandoned {
        claim: Vec<u8>,
        sender_id: String,
    },
    ResetPeerConnection {
        peer_id: PeerId,
    },
    RemovePeer {
        peer_id: PeerId,
        socket_addr: SocketAddr,
    },
    AddPeer(primitives::PeerId, SocketAddr, NodeType),
    DKGPartCommitment {
        part_commitment: Vec<u8>,
        sender_id: u16,
    },
    DKGPartAcknowledgement {
        curr_node_id: u16,
        sender_id: u16,
        ack: Vec<u8>,
    },

    Vote {
        vote: Vote,
        farmer_quorum_threshold: FarmerQuorumThreshold,
    },
    ForwardedTxn(TxnRecord),

    Ping(NodeId),

    #[default]
    Empty,
}
