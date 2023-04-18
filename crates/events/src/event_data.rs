use std::{collections::HashMap, net::SocketAddr};

use block::{Block, Conflict};
use ethereum_types::U256;
use messr::router::Router;
use primitives::{
    Address,
    ByteVec,
    FarmerQuorumThreshold,
    NodeIdx,
    NodeType,
    PeerId,
    QuorumPublicKey,
    QuorumType,
    RawSignature,
};
use quorum::quorum::Quorum;
use serde::{Deserialize, Serialize};
use telemetry::{error, info};
use tokio::sync::{
    broadcast::{self, Receiver, Sender},
    mpsc::{UnboundedReceiver, UnboundedSender},
};
use vrrb_core::{
    claim::Claim,
    txn::{TransactionDigest, Txn},
};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct PeerData {
    pub address: SocketAddr,
    pub node_type: NodeType,
    pub peer_id: PeerId,
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

#[derive(Debug, Deserialize, Serialize, Hash, Clone, PartialEq, Eq)]
pub struct VoteReceipt {
    /// The identity of the voter.
    pub farmer_id: Vec<u8>,
    pub farmer_node_id: NodeIdx,
    /// Partial Signature
    pub signature: RawSignature,
}

#[derive(Default, Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct QuorumCertifiedTxn {
    sender_farmer_id: Vec<u8>,
    /// All valid vote receipts
    votes: Vec<VoteReceipt>,
    pub txn: Txn,
    /// Threshold Signature
    signature: RawSignature,
}

impl QuorumCertifiedTxn {
    pub fn new(
        sender_farmer_id: Vec<u8>,
        votes: Vec<VoteReceipt>,
        txn: Txn,
        signature: RawSignature,
    ) -> QuorumCertifiedTxn {
        QuorumCertifiedTxn {
            sender_farmer_id,
            votes,
            txn,
            signature,
        }
    }
}

// `JobResult` is an enum that represents the possible results of a job that is
/// executed by a scheduler. It has two variants: `Votes` and `CertifiedTxn`.
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Hash, Clone)]
pub enum JobResult {
    Votes((Vec<Option<Vote>>, FarmerQuorumThreshold)),
    CertifiedTxn(
        Vec<Vote>,
        RawSignature,
        TransactionDigest,
        String,
        Vec<u8>,
        Txn,
    ),
}
