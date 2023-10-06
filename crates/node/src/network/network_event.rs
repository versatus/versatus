use std::net::SocketAddr;

use block::{Certificate, ConvergenceBlock};
use events::{AssignedQuorumMembership, Vote};
use hbbft::{
    crypto::PublicKeySet,
    sync_key_gen::{Ack, Part},
};
use mempool::TxnRecord;
use primitives::{ConvergencePartialSig, KademliaPeerId, NodeId, NodeType, PeerId, PublicKey};
use serde::{Deserialize, Serialize};
use signer::engine::QuorumData;
use vrrb_core::claim::Claim;

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
/// Represents data trasmitted over the VRRB network by nodes that participate
/// in it
pub enum NetworkEvent {
    ClaimCreated {
        node_id: NodeId,
        claim: Claim,
    },

    ClaimAbandoned {
        claim: Vec<u8>,
        sender_id: String,
    },

    PeerJoined {
        node_id: NodeId,
        node_type: NodeType,
        kademlia_peer_id: KademliaPeerId,
        udp_gossip_addr: SocketAddr,
        raptorq_gossip_addr: SocketAddr,
        kademlia_liveness_addr: SocketAddr,
        validator_public_key: PublicKey,
    },

    /// Peer was assigned to a specific quorum by a bootstrap node
    AssignmentToQuorumCreated {
        assigned_membership: AssignedQuorumMembership,
    },

    AssignmentToQuorumReceived {
        assigned_membership: AssignedQuorumMembership,
    },

    /// Peer is unresponsive or signaled its intent to leave the network
    PeerUnregistered {
        peer_id: PeerId,
        socket_addr: SocketAddr,
    },

    ForwardedTxn(Box<TxnRecord>),

    PartCommitmentCreated(NodeId, Part),
    PartCommitmentAcknowledged {
        node_id: NodeId,
        sender_id: NodeId,
        ack: Ack,
    },

    ConvergenceBlockCertified(ConvergenceBlock),
    ConvergenceBlockPartialSignComplete(ConvergencePartialSig),
    BroadcastCertificate(Certificate),
    BroadcastTransactionVote(Vote),
    Ping(NodeId),

    #[default]
    Empty,
}
