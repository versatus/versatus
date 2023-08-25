use std::net::SocketAddr;

use block::{
    header::BlockHeader, Block, BlockHash, Certificate, ConvergenceBlock, ProposalBlock, RefHash,
};
use ethereum_types::U256;
use hbbft::{
    crypto::PublicKeySet,
    sync_key_gen::{Ack, Part},
};
use primitives::{
    Address, Epoch, FarmerId, FarmerQuorumThreshold, IsTxnValid, NodeId, NodeIdx,
    ProgramExecutionOutput, PublicKey, PublicKeyShareVec, RawSignature, Round, Seed,
    TxnValidationStatus, ValidatorPublicKey, ValidatorPublicKeyShare,
};
use serde::{Deserialize, Serialize};
use vrrb_core::{
    claim::Claim,
    txn::{TransactionDigest, Txn},
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

    /// `NewTxnCreated(Txn)` is an event that is triggered when a new
    /// transaction is received from the rpc node and needs to be validated.
    /// The `Txn` parameter contains the details of the transaction
    /// that needs to be validated.
    NewTxnCreated(Txn),

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
    #[deprecated]
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

    /// `ClaimCreated(Claim)` represents a claim that is created for the node
    /// then has to be broadcasted.
    ClaimCreated(Claim),

    /// `ClaimReceived(Claim)` represents a claim emitted by another node
    ClaimReceived(Claim),

    /// `ClaimAbandoned(String,Vec<u8>)` represents a claim that turned out to
    /// be invalid.
    ClaimAbandoned(NodeId, Claim),

    /// A peer joined the network, should be added to the node's peer list
    PeerJoined(PeerData),

    /// A peer joined the network and was added to the node's peer list
    NodeAddedToPeerList(PeerData),

    /// `CreateAccountRequested((Address, AccountBytes))` is triggered when
    /// request for Account creation on the chain has been requested.
    CreateAccountRequested((Address, AccountBytes)),

    /// `AccountUpdateRequested((Address, AccountBytes))` is triggered when
    /// request for Account updation on the chain has been requested.
    AccountUpdateRequested((Address, AccountBytes)),

    /// `PeerSyncFailed(Vec<SocketAddr>)` is an event that is triggered when a
    /// peer address synchronization attempt fails. The `Vec<SocketAddr>`
    /// parameter contains a list of socket addresses of the peers
    /// that failed to synchronize their address.
    PeerSyncFailed(Vec<SocketAddr>),

    /// `BlockCreated(Block)` is an event that occurs whenever a block of any
    /// kind is created
    BlockCreated(Block),

    /// Event emitted by a bootrstrap QuorumModule to signal a node was assigned
    /// to a particular quorum
    QuorumMembershipAssigmentCreated(AssignedQuorumMembership),

    PartCommitmentCreated(NodeId, Part),

    PartCommitmentAcknowledged {
        /// The node whose commitment was acknowledged
        node_id: NodeId,
        /// The node who acknowledged the partial commitment
        sender_id: NodeId,
    },

    /// `HarvesterPublicKeyReceived(Vec<u8>)` is an event that carries a vector of bytes
    /// representing the public key of a harvester node. This event is used
    /// to communicate the public key of a harvester node to other nodes in
    /// the network.
    HarvesterPublicKeyReceived(PublicKeySet),

    /// This events triggers the generation of a certificate for a given transaction
    TransactionCertificateRequested {
        votes: Vec<Vote>,
        txn_id: TransactionDigest,
        quorum_key: PublicKeyShareVec,
        farmer_id: NodeId,
        txn: Txn,
        quorum_threshold: FarmerQuorumThreshold,
    },

    /// This event is emitted whenever a transaction is certified by a Farmer Quorum
    TransactionCertificateCreated {
        votes: Vec<Vote>,
        signature: RawSignature,
        digest: TransactionDigest,
        /// OUtput of the program executed
        execution_result: ProgramExecutionOutput,
        farmer_id: NodeId,
        txn: Box<Txn>,
        is_valid: TxnValidationStatus,
    },

    MinerElectionStarted(BlockHeader),

    MinerElected((U256, Claim)),

    ProposalBlockCreated(ProposalBlock),

    ConvergenceBlockCreated(ConvergenceBlock),

    ConvergenceBlockCertified(ConvergenceBlock),

    QuorumElectionStarted(BlockHeader),

    // NOTE: replaces Event::Farm and pushes txns to the scheduler instead of having it pull them
    TxnsReadyForProcessing(Vec<Txn>),

    TxnsValidated {
        votes: Vec<Option<Vote>>,
        quorum_threshold: FarmerQuorumThreshold,
    },

    /// `ProposalBlockMineRequestCreated` triggers the mining of a proposal
    /// block by a farmer node after every `X` seconds. The proposal block
    /// contains a list of transactions that have been validated and certified
    /// by the farmer node
    ProposalBlockMineRequestCreated {
        ref_hash: RefHash,
        round: Round,
        epoch: Epoch,
        claim: Claim,
    },

    ConvergenceBlockSignatureRequested(ConvergenceBlock),

    /// `ConvergenceBlockPartialSignatureCreated` is an event that is triggered
    /// when a node has partially signed a convergence block. The
    /// `JobResult` parameter contains the result of the partial signing
    /// process, which includes the partial signature and the public key share
    /// used to verify it. This event is used to communicate the partial
    /// signature to other nodes in the network, so that they can aggregate
    /// it with their own partial signatures to create a complete signature for
    /// the convergence block,also it adds the partial signature to
    /// certificate cache
    ConvergenceBlockPartialSignatureCreated {
        block_hash: BlockHash,
        public_key_share: ValidatorPublicKeyShare,
        partial_signature: RawSignature,
    },

    /// `ConvergenceBlockPrecheckRequested` is a function
    /// used to precheck a convergence block before it is signed and added
    /// to the blockchain. This precheck process involves verifying the validity
    /// of the convergence block. The verification includes checking that
    /// the block hashes correctly reference proposal block hashes,
    /// as well as verifying the claim hashes and transaction hashes associated
    /// with the convergence block.
    ConvergenceBlockPrecheckRequested {
        convergence_block: ConvergenceBlock,
        block_header: BlockHeader,
    },

    /// `ConvergenceBlockPeerSignatureRequested` is an event that is used to create an
    /// aggregated signatures out of  a partial signature shares from peers
    ConvergenceBlockPeerSignatureRequested {
        node_id: NodeId,
        block_hash: BlockHash,
        public_key_share: PublicKeyShareVec,
        partial_signature: RawSignature,
    },

    Ping(NodeId),

    // TODO: refactor all the events below
    // ==========================================================================
    ///
    ///
    /// `UpdateState` is an event that triggers the update of the node's state
    /// to a new block hash. This event is used to update the node's state
    /// after a last new convergence block has been certified .
    UpdateState(BlockHash),

    /// `ConvergenceBlockPartialSign(JobResult)` is an event that is triggered
    /// when a node has partially signed a convergence block. The
    /// `JobResult` parameter contains the result of the partial signing
    /// process, which includes the partial signature and the public key share
    /// used to verify it. This event is used to communicate the partial
    /// signature to other nodes in the network, so that they can aggregate
    /// it with their own partial signatures to create a complete signature for
    /// the convergence block,also it adds the partial signature to
    /// certificate cache
    ConvergenceBlockPartialSign(JobStatus),

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
    BlockCertificateCreated(Certificate),
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
