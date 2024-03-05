use block::GenesisReceiver;
use block::{header::BlockHeader, Block, BlockHash, Certificate, ConvergenceBlock, ProposalBlock};
use ethereum_types::U256;
use hbbft::sync_key_gen::Ack;
use hbbft::{crypto::PublicKeySet, sync_key_gen::Part};
use primitives::{
    Address, ConvergencePartialSig, FarmerQuorumThreshold, NodeId, Signature, RUNTIME_TOPIC_STR,
};

use serde::{Deserialize, Serialize};
use vrrb_core::claim::Claim;
use vrrb_core::transactions::{TransactionDigest, TransactionKind};

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
    NewTxnCreated(TransactionKind),

    /// `TxnValidated(Txn)` is an event that is triggered when a transaction has
    /// been validated by the validator module. The `Txn` parameter contains
    /// the details of the validated transaction. This event can be used to
    /// perform further actions on the validated transaction, such as removing
    /// it from pending mempool and adding it into the TransactionStore
    TxnValidated(TransactionKind),

    /// `TxnAddedToMempool(TransactionDigest)` is an event that is triggered
    /// when a transaction has been added to the mempool. The
    /// `TransactionDigest` parameter contains a digest of the transaction
    /// that has been added to the mempool.
    TxnAddedToMempool(TransactionDigest),

    /// `BlockReceived(NodeId, Block)` represents a block that has been received from
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

    /// `BlockCreated(Block)` is an event that occurs whenever a block of any
    /// kind is created
    BlockCreated(Block),

    /// Event emitted by a bootrstrap QuorumModule to signal a node was assigned
    /// to a particular quorum
    QuorumMembershipAssigmentCreated(AssignedQuorumMembership),

    /// Event emitted by a bootrstrap QuorumModule to signal a group of nodes were assigned
    /// to a particular quorum
    QuorumMembershipAssigmentsCreated(Vec<AssignedQuorumMembership>),

    PartCommitmentCreated(NodeId, Part),

    PartCommitmentAcknowledged {
        /// The node whose commitment was acknowledged
        node_id: NodeId,
        /// The node who acknowledged the partial commitment
        sender_id: NodeId,

        ack: Ack,
    },

    /// `HarvesterPublicKeyReceived(Vec<u8>)` is an event that carries a vector of bytes
    /// representing the public key of a harvester node. This event is used
    /// to communicate the public key of a harvester node to other nodes in
    /// the network.
    HarvesterPublicKeyReceived(PublicKeySet),

    MinerElectionStarted(BlockHeader),

    MinerElected((U256, Claim)),

    GenesisMinerElected {
        genesis_receivers: Vec<GenesisReceiver>,
    },

    ConvergenceBlockCertified(ConvergenceBlock),

    QuorumElectionStarted(BlockHeader),

    TransactionsValidated {
        vote: Vote,
        quorum_threshold: FarmerQuorumThreshold,
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

    // TODO: refactor all the events below
    // ==========================================================================
    ///
    ///
    /// `UpdateState` is an event that triggers the update of the node's state
    /// to a new block hash. This event is used to update the node's state
    /// after a last new convergence block has been certified.
    UpdateState(ConvergenceBlock),

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
    ConvergenceBlockPartialSignComplete(ConvergencePartialSig),

    /// `SignConvergenceBlock(ConvergenceBlock)` is an event that triggers the
    /// signing of a convergence block by the node. This is done by sending
    /// a Job to the scheduler
    SignConvergenceBlock(ConvergenceBlock),

    /// `BlockCertificate(Certificate)` is an event that carries a `Certificate`
    /// object representing a proof that a block has been certified by a
    /// quorum. This certificate is then added to convergence block .
    BlockCertificateCreated(Certificate),
    QuorumFormed,
    HarvesterSignatureReceived(BlockHash, NodeId, Signature),
    BroadcastCertificate(Certificate),
    BroadcastTransactionVote(Vote),
    BlockAppended(String),
    BuildProposalBlock(ConvergenceBlock),
    BroadcastProposalBlock(ProposalBlock),
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
        match &evt {
            Event::Stop => messr::Message::stop_signal(None),
            Event::CreateAccountRequested(_)
            | Event::NewTxnCreated(_)
            | Event::TxnAddedToMempool(_) => {
                messr::Message::new(Some(RUNTIME_TOPIC_STR.into()), evt)
            }
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
