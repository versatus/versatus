use std::net::SocketAddr;

use block::BlockHash;
use hbbft::crypto::PublicKeyShare;
use primitives::{
    ByteVec, FarmerId, FarmerQuorumThreshold, IsTxnValid, KademliaPeerId, NodeId, NodeIdx,
    NodeType, QuorumKind, RawSignature,
};
use serde::{Deserialize, Serialize};
use vrrb_core::txn::{TransactionDigest, Txn};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct PeerData {
    pub node_id: NodeId,
    pub node_type: NodeType,
    pub kademlia_peer_id: KademliaPeerId,
    pub udp_gossip_addr: SocketAddr,
    pub raptorq_gossip_addr: SocketAddr,
    pub kademlia_liveness_addr: SocketAddr,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct SyncPeerData {
    pub address: SocketAddr,
    pub raptor_udp_port: u16,
    pub quic_port: u16,
    pub node_type: NodeType,
}

// NOTE: naming convention for events goes as follows:
// <Subject><Verb, in past tense>, e.g. ObjectCreated
// TODO: Replace Vec<u8>'s with proper data structs in enum wariants
// once definitions of those are moved into primitives.

#[derive(Debug, Deserialize, Serialize, Hash, Clone, PartialEq, Eq)]
pub struct Vote {
    /// The identity of the voter.
    pub farmer_id: Vec<u8>,
    pub farmer_node_id: NodeIdx,
    /// Partial Signature
    pub signature: RawSignature,
    pub txn: Txn,
    pub quorum_public_key: Vec<u8>,
    pub quorum_threshold: usize,
    pub is_txn_valid: bool,
    // May want to serialize this as a vector of bytes
    pub execution_result: Option<String>,
}

pub type SerializedConvergenceBlock = ByteVec;

#[derive(Debug, Deserialize, Serialize, Hash, Clone, PartialEq, Eq)]
pub struct BlockVote {
    pub harvester_id: Vec<u8>,
    pub harvester_node_id: NodeIdx,
    pub signature: RawSignature,
    pub convergence_block: SerializedConvergenceBlock,
    pub quorum_public_key: Vec<u8>,
    pub quorum_threshold: usize,
    // May want to serialize this as a vector of bytes
    pub execution_result: Option<String>,
}

// `JobResult` is an enum that represents the possible results of a job that is
/// executed by a scheduler. It has two variants: `Votes` and `CertifiedTxn`.
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Hash, Clone)]
pub enum JobResult {
    /// `Votes((Vec<Option<Vote>>, FarmerQuorumThreshold))` is type of
    /// `JobResult` which denotes votes from farmers for a particular txn.
    /// while the `FarmerQuorumThreshold` specifies the minimum number of votes
    /// required for the job to be considered successful.
    Votes((Vec<Option<Vote>>, FarmerQuorumThreshold)),
    /// `CertifiedTxn` is a variant of the `JobResult` enum that represents the
    /// result of a job that certifies a transaction. It contains the
    /// following fields:
    /// - `Vec<Vote>`: a vector of votes that were cast for the transaction.
    /// - `RawSignature`: the signature of the farmer  who voted on the txn
    /// - `TransactionDigest`: the digest of the transaction.
    /// - `String`: the execution result of the transaction.
    /// - `FarmerId`: id of the farmer
    /// - `Txn`: the transaction itself.
    /// - `IsTxnValid`: a boolean indicating whether the transaction is valid or
    ///   not.
    CertifiedTxn(
        Vec<Vote>,
        RawSignature,
        TransactionDigest,
        String,
        FarmerId,
        Box<Txn>,
        IsTxnValid,
    ),
    /// `ConvergenceBlockPartialSign(BlockHash,RawSignature)` is a variant of
    /// the `JobResult` enum that represents the result of a job that
    /// partially signs a convergence block. It contains the
    /// following fields:
    /// - `BlockHash`: the hash of the convergence block being partially signed.
    /// - `RawSignature`: the partial signature of the harvester who partially
    ///   signed the convergence
    /// block.
    ConvergenceBlockPartialSign(BlockHash, PublicKeyShare, RawSignature),
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Hash, Clone)]
pub struct AssignedQuorumMembership {
    pub node_id: NodeId,
    pub kademlia_peer_id: KademliaPeerId,
    pub quorum_kind: QuorumKind,
    pub peers: Vec<PeerData>,
}
