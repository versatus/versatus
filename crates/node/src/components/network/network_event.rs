use std::net::SocketAddr;

use events::Vote;
use mempool::TxnRecord;
use primitives::{
    AckBytes, CurrentNodeId, FarmerQuorumThreshold, KademliaPeerId, NodeId, NodeType,
    PartCommitmentBytes, PeerId, SenderId,
};
use serde::{Deserialize, Serialize};
use vrrb_core::claim::Claim;

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

    ClaimCreated {
        node_id: NodeId,
        claim: Claim,
    },

    ClaimAbandoned {
        claim: Vec<u8>,
        sender_id: String,
    },

    ResetPeerConnection {
        peer_id: PeerId,
    },

    PeerJoined {
        node_id: NodeId,
        node_type: NodeType,
        kademlia_peer_id: KademliaPeerId,
        udp_gossip_addr: SocketAddr,
        raptorq_gossip_addr: SocketAddr,
        kademlia_liveness_addr: SocketAddr,
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

    PartMessage(SenderId, PartCommitmentBytes),

    Ack(CurrentNodeId, SenderId, AckBytes),

    #[default]
    Empty,
}
