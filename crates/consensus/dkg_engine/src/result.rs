use hbbft::sync_key_gen::Part;
use thiserror::Error;

/// List of all possible errors related to synchronous dkg generation .
#[derive(Error, Debug)]
pub enum DkgError {
    #[error("Not enough peer public messages keys to start DKG process")]
    NotEnoughPeerPublicKeys,
    #[error("Sync key Generation instance not created .")]
    SyncKeyGenInstanceNotCreated,
    #[error("Not enough part messages received")]
    NotEnoughPartMsgsReceived,
    #[error("Atleast t+1 parts needs to be completed for DKG generation to happen")]
    NotEnoughPartsCompleted,
    #[error("Not enough ack messages received")]
    NotEnoughAckMsgsReceived,
    #[error("Partial Committment not generated")]
    PartCommitmentNotGenerated,
    #[error("Partial Committment missing for node with index {0}")]
    PartMsgMissingForNode(u16),
    #[error("Partial Message already acknowledge for node with index {0}")]
    PartMsgAlreadyAcknowledge(u16),
    #[error("Invalid Part Message Error: {0}")]
    InvalidPartMessage(String),
    #[error("Invalid Ack Message Error: {0}")]
    InvalidAckMessage(String),
    #[error("Unknown error occurred while synckeygen process , Details :{0} ")]
    SyncKeyGenError(String),
    #[error("Invalid Key {0} Value {1}")]
    ConfigInvalidValue(String, String),
    #[error("Only MasterNode should participate in DKG generation process")]
    InvalidNode,
    #[error("All participants of Quorum need to actively participate in DKG")]
    ObserverNotAllowed,
    #[error("Unknown Error: {0}")]
    Unknown(String),
}

#[derive(Debug)]
pub enum DkgResult {
    PartMessageGenerated(u16, Part),
    PartMessageAcknowledged,
    AllAcksHandled,
    KeySetsGenerated,
}

pub type Result<T> = std::result::Result<T, DkgError>;
