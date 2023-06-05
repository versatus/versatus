use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
};

use block::{
    header::BlockHeader,
    Block,
    BlockHash,
    Certificate,
    Conflict,
    ConvergenceBlock,
    ProposalBlock,
    RefHash,
};
use ethereum_types::U256;
use mempool::TxnRecord;
use primitives::{
    Address,
    Epoch,
    FarmerQuorumThreshold,
    GroupPublicKey,
    HarvesterQuorumThreshold,
    NodeIdx,
    PublicKeyShareVec,
    QuorumPublicKey,
    QuorumSize,
    RawSignature,
    Round,
    Seed,
};
use quorum::quorum::Quorum;
use serde::{Deserialize, Serialize};
use vrrb_core::{
    claim::Claim,
    txn::{QuorumCertifiedTxn, TransactionDigest, Txn},
};

use crate::event_data::*;

pub type AccountBytes = Vec<u8>;
pub type BlockBytes = Vec<u8>;
pub type HeaderBytes = Vec<u8>;
pub type ConflictBytes = Vec<u8>;
pub type MinerClaim = Claim;

#[derive(Default, Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Event {
    #[default]
    NoOp,
    Stop,

    /// Swarm module events
    FetchPeers(usize),
    DHTStoreRequest(String, String),

    /// New txn came from network, requires validation
    NewTxnCreated(Txn),

    /// `ForwardTxn` is an event that is used to forward a transaction(That is
    /// not meant to be processed by current quorum) to a list of peers.
    ForwardTxn((TxnRecord, Vec<SocketAddr>)),
    /// Single txn validated
    TxnValidated(Txn),
    /// Batch of validated txns
    TxnBatchValidated(Vec<TransactionDigest>),
    TxnAddedToMempool(TransactionDigest),
    MempoolSizeThesholdReached {
        cutoff_transaction: TransactionDigest,
    },
    BlockReceived(Block),
    BlockConfirmed(Vec<u8>),
    ClaimCreated(Vec<u8>),
    ClaimProcessed(Vec<u8>),
    UpdateLastBlock(Vec<u8>),
    ClaimAbandoned(String, Vec<u8>),
    SlashClaims(Vec<String>),
    CheckAbandoned,

    //   SyncNeighbouringFarmerQuorum(HashMap<GroupPublicKey, HashSet<SocketAddr>>),
    SyncPeers(Vec<SyncPeerData>),
    PeerRequestedStateSync(PeerData),

    //Event to tell Farmer node to sign the Transaction
    //the validator module has validated this transaction
    ValidTxn(TransactionDigest),
    /// A peer joined the network, should be added to the node's peer list
    PeerJoined(PeerData),

    /// Peer abandoned the network. Should be removed from the node's peer list
    PeerLeft(PeerData),

    /// A Event to start the DKG process.
    DkgInitiate,

    /// A command to  ack Part message of  sender .
    AckPartCommitment(u16),

    /// Event to broadcast Part Message
    PartMessage(u16, Vec<u8>),

    /// Event to broadcast Part Message
    SendPartMessage(u16, Vec<u8>),

    /// A command to  send ack of Part message of sender by current Node.
    SendAck(u16, u16, Vec<u8>),

    /// A command to handle all the acks received by the node.
    HandleAllAcks,

    /// Used to generate the public key set& Distrbuted Group Public Key for the
    /// node.
    GenerateKeySet,
    HarvesterPublicKey(Vec<u8>),
    Farm,
    Vote(Vote, FarmerQuorumThreshold),
    MineProposalBlock(RefHash, Round, Epoch, Claim),
    PullQuorumCertifiedTxns(usize),
    QuorumCertifiedTxns(QuorumCertifiedTxn),

    ConfirmedTxns(Vec<(String, QuorumPublicKey)>),

    CreateAccountRequested((Address, AccountBytes)),
    AccountCreated(Address),

    AccountUpdateRequested((Address, AccountBytes)),
    UpdatedAccount(AccountBytes),
    // May want to just use the `BlockHeader` struct to reduce
    // the overhead of deserializing
    MinerElection(HeaderBytes),
    ElectedMiner((U256, Claim)),
    QuorumElection(HeaderBytes),
    ElectedQuorum(Quorum),
    MinedBlock(Block),
    // May want to just use the ConflictList & `BlockHeader` types
    // to reduce the overhead of deserializing
    ConflictResolution(ConflictBytes, HeaderBytes),
    ResolvedConflict(Conflict),
    EmptyPeerSync,
    PeerSyncFailed(Vec<SocketAddr>),
    ProcessedVotes(JobResult),
    ConvergenceBlockPartialSign(JobResult),
    FarmerQuorum(QuorumSize, FarmerQuorumThreshold),
    HarvesterQuorum(QuorumSize, HarvesterQuorumThreshold),
    UpdateState(BlockHash),
    CertifiedTxn(JobResult),
    AddHarvesterPeer(SocketAddr),
    RemoveHarvesterPeer(SocketAddr),
    CheckConflictResolution((Vec<ProposalBlock>, Round, Seed, ConvergenceBlock)),
    SignConvergenceBlock(ConvergenceBlock),
    PeerConvergenceBlockSign(NodeIdx, BlockHash, PublicKeyShareVec, RawSignature),
    SendPeerConvergenceBlockSign(NodeIdx, BlockHash, PublicKeyShareVec, RawSignature),
    SendBlockCertificate(Certificate),
    BlockCertificate(Certificate),
    PrecheckConvergenceBlock(ConvergenceBlock, BlockHeader),
}

impl From<&theater::Message> for Event {
    fn from(msg: &theater::Message) -> Self {
        serde_json::from_slice(&msg.data).unwrap_or_default()
    }
}

impl From<theater::Message> for Event {
    fn from(msg: theater::Message) -> Self {
        serde_json::from_slice(&msg.data).unwrap_or_default()
    }
}

impl From<Vec<u8>> for Event {
    fn from(data: Vec<u8>) -> Self {
        serde_json::from_slice(&data).unwrap_or_default()
    }
}

impl From<Event> for Vec<u8> {
    fn from(evt: Event) -> Self {
        serde_json::to_vec(&evt).unwrap_or_default()
    }
}

impl From<Event> for messr::Message<Event> {
    fn from(evt: Event) -> Self {
        match evt {
            Event::Stop => messr::Message::stop_signal(None),
            _ => messr::Message::new(None, evt),
        }
    }
}

impl From<messr::MessageData<Event>> for Event {
    fn from(md: messr::MessageData<Event>) -> Self {
        match md {
            messr::MessageData::Data(evt) => evt,
            messr::MessageData::StopSignal => Event::Stop,
            _ => Event::NoOp,
        }
    }
}

impl From<messr::Message<Event>> for Event {
    fn from(message: messr::Message<Event>) -> Self {
        let md = message.data;
        match md {
            messr::MessageData::Data(evt) => evt,
            messr::MessageData::StopSignal => Event::Stop,
            _ => Event::NoOp,
        }
    }
}
