use std::net::SocketAddr;

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
pub type Count = usize;

#[derive(Default, Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Event {
    /// `NoOp` is short for "no operation" and is a default case for the `Event`
    /// enum. It is used when no other event is applicable or when an event
    /// is not explicitly specified. It does not perform any action and is
    /// essentially a placeholder.
    #[default]
    NoOp,

    /// `Stop` is an event that signals the node to stop its execution and
    /// handling of events.
    Stop,

    /// `FetchPeers(Count),` is an event that is triggered when something
    /// nearest neighbors within smart are fetched.
    FetchPeers(Count),

    /// ` DHTStoreRequest(String, String)` is an event that is triggered when
    /// something is stored in swarm dht.
    DHTStoreRequest(String, String),

    /// `NewTxnCreated(Txn)` is an event that is triggered when a new
    /// transaction is received from the rpc node and needs to be validated.
    /// The `Txn` parameter contains the details of the transaction
    /// that needs to be validated.
    NewTxnCreated(Txn),

    /// `ForwardTxn` is an event that is used to forward a transaction that is
    /// not meant to be processed by the current quorum to a list of peers.
    ForwardTxn((TxnRecord, Vec<SocketAddr>)),

    /// `TxnValidated(Txn)` is an event that is triggered when a transaction has
    /// been validated by the validator module. The `Txn` parameter contains
    /// the details of the validated transaction. This event can be used to
    /// perform further actions on the validated transaction, such as removing
    /// it from pending mempool and adding it into the TransactionStore
    TxnValidated(Txn),

    /// `TxnAddedToMempool(TransactionDigest)` is an event that is triggered
    /// when a transaction has been added to the mempool. The
    /// `TransactionDigest` parameter contains a digest of the transaction
    /// that has been added to the mempool.
    TxnAddedToMempool(TransactionDigest),

    /// `MempoolSizeThesholdReached` is an event that is triggered when the size
    /// of the confirmed transaction mempool reaches a certain threshold.
    /// The `cutoff_transaction` parameter contains the digest of
    /// the transaction that is used as a cutoff point for removing transactions
    /// from the mempool. This event is used by Harvester to trigger build
    /// of proposal blocks when confirmed transactions in pool reaches a
    /// threshold.
    MempoolSizeThesholdReached {
        cutoff_transaction: TransactionDigest,
    },

    /// `BlockReceived(Block)` represents a block that has been received from
    /// peers in the network. The block can be a genesis block, a proposal
    /// block, or a convergence block.
    BlockReceived(Block),

    //BlockConfirmed — Should we broadcast convergence block and certificate to all nodes
    // separately?
    BlockConfirmed(Vec<u8>),

    /// `ClaimCreated(Vec<u8>)` represents a claim that is created for the node
    /// then has to be broadcasted.
    ClaimCreated(Vec<u8>),

    /// `ClaimAbandoned(String,Vec<u8>)` represents a claim that turned out to
    /// be invalid.
    ClaimAbandoned(String, Vec<u8>),

    ///  `SlashClaims`represent slashing of claims of the nodes.More information
    /// yet to be added.
    SlashClaims(Vec<String>),

    /// `SyncPeers(Vec<SyncPeerData>)` is an event that is triggered
    /// periodically, typically every `X` seconds, to synchronize the peers
    /// of the current Quorum with the Rendezvous nodes. It also includes
    /// the synchronization of peers from neighboring farmer quorums.
    SyncPeers(Vec<SyncPeerData>),

    //Event to tell Farmer node to sign the Transaction
    //the validator module has validated this transaction
    ValidTxn(TransactionDigest),

    /// A peer joined the network, should be added to the node's peer list
    PeerJoined(PeerData),

    /// `DkgInitiate` is an event that is triggered to initiate the distributed
    /// key generation process for the Quorum after the quorum has been
    /// elected.
    DkgInitiate,

    /// `AckPartCommitment(u16)` is an event that is triggered when part
    /// commitment has been received from the network, and now it has to be
    /// acknowledged
    AckPartCommitment(u16),

    /// `PartMessage(u16,Vec<u8>)` is an event that is triggered when part
    /// message has been received from the network, and has to be recorded
    /// in the part message store.
    PartMessage(u16, Vec<u8>),

    /// `SendAck(u16,u16,Vec<u8>)` is an event that is triggered when part
    /// commitment has been acknowledged by the current node ,it has to be
    /// broadcasted to the other members of the Elected Quorum.
    SendAck(u16, u16, Vec<u8>),

    /// `HandleAllAcks` is an event that is triggered to handle all the
    /// acknowledgments of partial commitments given by Nodes elected for
    /// Quorum.
    HandleAllAcks,

    /// `GenerateKeySet` is an event that triggers the generation of a public
    /// key set from which the distributed group public key can be generated
    /// for the Quorum. The generated key set serves as the foundation for
    /// establishing trustworthy and robust Quorum.
    GenerateKeySet,

    /// `HarvesterPublicKey(Vec<u8>)` is an event that carries a vector of bytes
    /// representing the public key of a harvester node. This event is used
    /// to communicate the public key of a harvester node to other nodes in
    /// the network.
    HarvesterPublicKey(Vec<u8>),

    /// The`Farm` event represents the process of fetching a batch of
    /// transactions from the transaction mem-pool and sending them to the
    /// scheduler. The scheduler then validates and votes on these
    /// transactions.
    Farm,

    /// `Vote(Vote, FarmerQuorumThreshold)` is an event that triggered when vote
    /// on transaction is received from neighboring peers of Quorum.
    Vote(Vote, FarmerQuorumThreshold),

    /// `MineProposalBlock` is an event that triggers the mining of a proposal
    /// block by a farmer node after every `X` seconds. The proposal block
    /// contains a list of transactions that have been validated and certified
    /// by the farmer node
    MineProposalBlock(RefHash, Round, Epoch, Claim),

    /// `CreateAccountRequested((Address, AccountBytes))` is triggered when
    /// request for Account creation on the chain has been requested.
    CreateAccountRequested((Address, AccountBytes)),

    /// `AccountUpdateRequested((Address, AccountBytes))` is triggered when
    /// request for Account updation on the chain has been requested.
    AccountUpdateRequested((Address, AccountBytes)),

    /// `MinerElection(HeaderBytes)` is an event that is triggered after the
    /// last convergence block is mined and the proposal blocks are built.
    MinerElection(HeaderBytes),

    /// `ElectedMiner((U256, Claim))` is an event that is triggered after the
    /// last convergence block is mined and the elected miner mines a new
    /// convergence block.
    ElectedMiner((U256, Claim)),

    /// `QuorumElection(HeaderBytes)` is an event that is triggered to initiate
    /// the Quorum Election process, once the elected candidates have
    /// broadcasted their claims to each other.
    QuorumElection(HeaderBytes),

    /// `ElectedQuorum(Quorum)` is an event that is triggered when Quorum is
    /// successfully elected.
    ElectedQuorum(Quorum),

    /// `MinedBlock(Block)` is an event that occurs when either the harvester
    /// has successfully mined a `ProposalBlock` or the miner has
    /// successfully mined a `ConvergenceBlock`.
    MinedBlock(Block),

    /// `EmptyPeerSync` is an event that is triggered when a current node has no
    /// peers.
    EmptyPeerSync,

    /// `PeerSyncFailed(Vec<SocketAddr>)` is an event that is triggered when a
    /// peer address synchronization attempt fails. The `Vec<SocketAddr>`
    /// parameter contains a list of socket addresses of the peers
    /// that failed to synchronize their address.
    PeerSyncFailed(Vec<SocketAddr>),

    /// `ProcessedVotes` is an event that is triggered when a batch of
    /// transactions has been validated and then voted by node's scheduler.
    /// The `JobResult` parameter contains the result of the processing, which
    /// includes the vote . This vote is then broadcasted to other peers to
    /// certify the transaction once threshold is reached.
    ProcessedVotes(JobResult),

    /// `UpdateState` is an event that triggers the update of the node's state
    /// to a new block hash. This event is used to update the node's state
    /// after a last new convergence block has been certified .
    UpdateState(BlockHash),

    /// `CertifiedTxn(JobResult)` is an event that is triggered when a
    /// transaction has been certified by a quorum using Job Scheduler. The
    /// `JobResult` parameter contains the result of the certification process,
    /// which includes the certified transaction and the certificate that
    /// proves its validity.
    CertifiedTxn(JobResult),

    /// `AddHarvesterPeer(SocketAddr)` is an event that is used to add a new
    /// harvester peer to the farmer node's list of harvester peers.
    AddHarvesterPeer(SocketAddr),

    /// `RemoveHarvesterPeer(SocketAddr)` is an event that is used to remove a
    /// harvester peer from the farmer node's list of harvester peers.
    RemoveHarvesterPeer(SocketAddr),

    /// `ConvergenceBlockPartialSign(JobResult)` is an event that is triggered
    /// when a node has partially signed a convergence block. The
    /// `JobResult` parameter contains the result of the partial signing
    /// process, which includes the partial signature and the public key share
    /// used to verify it. This event is used to communicate the partial
    /// signature to other nodes in the network, so that they can aggregate
    /// it with their own partial signatures to create a complete signature for
    /// the convergence block,also it adds the partial signature to
    /// certificate cache
    ConvergenceBlockPartialSign(JobResult),

    /// `CheckConflictResolution` is an event that triggers the checking of a
    /// proposed conflict resolution.The event is used to initiate the
    /// conflict resolution process by checking if the proposed conflict
    /// resolution is valid. This involves verifying that the proposal
    /// blocks are valid and that they correctly reference the convergence
    /// block. If the proposed conflict resolution is valid, the event
    /// triggers the signing of the convergence block and the creation of a
    /// certificate to prove its validity.
    CheckConflictResolution((Vec<ProposalBlock>, Round, Seed, ConvergenceBlock)),

    /// `SignConvergenceBlock(ConvergenceBlock)` is an event that triggers the
    /// signing of a convergence block by the node. This is done by sending
    /// a Job to the scheduler
    SignConvergenceBlock(ConvergenceBlock),

    /// `PeerConvergenceBlockSign` is an event that is used to create an
    /// aggregated signatures out of  a partial signature shares from peers
    PeerConvergenceBlockSign(NodeIdx, BlockHash, PublicKeyShareVec, RawSignature),

    /// `SendPeerConvergenceBlockSign` is an event that triggers the sharing of
    /// a convergence block partial signature with other peers.
    SendPeerConvergenceBlockSign(NodeIdx, BlockHash, PublicKeyShareVec, RawSignature),

    /// `SendBlockCertificate(Certificate)` is an event that triggers the
    /// sending of a `Certificate` object representing a proof that a block
    /// has been certified by a quorum  in the network. This event is used
    /// to communicate the convergence block certification to other nodes in the
    /// network.
    SendBlockCertificate(Certificate),

    /// `BlockCertificate(Certificate)` is an event that carries a `Certificate`
    /// object representing a proof that a block has been certified by a
    /// quorum. This certificate is then added to convergence block .
    BlockCertificate(Certificate),

    /// `PrecheckConvergenceBlock(ConvergenceBlock, BlockHeader)` is a function
    /// used to precheck a convergence block before it is signed and added
    /// to the blockchain. This precheck process involves verifying the validity
    /// of the convergence block. The verification includes checking that
    /// the block hashes correctly reference proposal block hashes,
    /// as well as verifying the claim hashes and transaction hashes associated
    /// with the convergence block.
    PrecheckConvergenceBlock(ConvergenceBlock, BlockHeader),

    /// Bogus event meant for experimentation. Remove soon
    Other(String),
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
