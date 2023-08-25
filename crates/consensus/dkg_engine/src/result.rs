use std::collections::{BTreeMap, HashMap};

use hbbft::{
    crypto::{PublicKey, PublicKeySet, SecretKey, SecretKeyShare},
    sync_key_gen::{Ack, Part, SyncKeyGen},
};
use primitives::{NodeId, NodeIdx, NodeType};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, DkgError>;

/// List of all possible errors related to synchronous dkg generation .
#[derive(Error, Debug)]
pub enum DkgError {
    #[error("Not enough peer public keys to create partial commitment")]
    NotEnoughPeerPublicKeys,
    #[error("Sync key Generation instance not created.")]
    SyncKeyGenInstanceNotCreated,
    #[error("Not enough part messages received")]
    NotEnoughPartMsgsReceived,
    #[error("Atleast t+1 parts needs to be completed for DKG generation to happen")]
    NotEnoughPartsCompleted,
    #[error("Not enough ack messages received")]
    NotEnoughAckMsgsReceived,
    #[error("Partial commitment not generated")]
    PartCommitmentNotGenerated,
    #[error("Partial Committment missing for node with index {0}")]
    PartMsgMissingForNode(NodeId),
    #[error("Partial Message already acknowledge for node with index {0}")]
    PartMsgAlreadyAcknowledged(NodeId),
    #[error("Invalid Part Message Error: {0}")]
    InvalidPartMessage(String),
    #[error("Invalid Ack Message Error: {0}")]
    InvalidAckMessage(String),
    #[error("Unknown error occurred while synckeygen process: {0}")]
    SyncKeyGenError(String),
    #[error("Invalid Key {0}  Value {1}")]
    ConfigInvalidValue(String, String),
    #[error("Only MasterNode should participate in DKG generation process")]
    InvalidNode,
    #[error("All participants of Quorum need to actively participate in DKG")]
    ObserverNotAllowed,
    #[error("Unknown Error: {0}")]
    Unknown(String),
}
