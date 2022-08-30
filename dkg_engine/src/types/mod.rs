pub mod config;
use crate::types::config::ThresholdConfig;
use hbbft::{
    crypto::{PublicKey, PublicKeySet, SecretKeyShare, SecretKey},
    sync_key_gen::{Part, SyncKeyGen, Ack},
};
use node::node::Node;
use rand::rngs::OsRng;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, RwLock};
use thiserror::Error;

/// `DkgEngine` is a struct that holds entry point for initiating DKG
///
/// Properties:
///
/// * `node_info`: This is a struct that contains information about the node. It contains the node type
/// (leader or follower) and the node index.
/// * `threshold_config`: This is the configuration for the threshold scheme. It contains the number of
/// nodes in the network, the threshold, and the number of nodes that are required to be online for the
/// threshold scheme to work.
/// * `dkg_state`: This is the state of the DKG protocol. It is a struct that contains the following
/// properties:
pub struct DkgEngine {
    /// To Get Info like Node Type and Node Idx
    pub node_info: Arc<RwLock<Node>>,

    /// For DKG (Can be extended for heirarchical DKG)
    pub threshold_config: ThresholdConfig,

    /// state information related to dkg process
    pub dkg_state: DkgState,
}

impl DkgEngine {
    pub fn new(
        node_info: Arc<RwLock<Node>>,
        threshold_config: ThresholdConfig
    ) -> DkgEngine {
        let secret_key:SecretKey=rand::random();
        DkgEngine {
            node_info,
            threshold_config,
            dkg_state:DkgState {
                part_message_store: HashMap::new(),
                ack_message_store: HashMap::new(),
                peer_public_keys: BTreeMap::new(),
                public_key_set: None,
                secret_key_share: None,
                sync_key_gen: None,
                random_number_gen: None,
                secret_key
            }
        }
    }
}

/// `DkgState` is a struct that contains a vector of `Part` messages, a vector of `Ack` messages, a map
/// of `PublicKey`s, an optional `PublicKeySet`, an optional `SecretKeyShare`, and an optional
/// `SyncKeyGen`.
///
/// Properties:
///
/// * `part_message_store`: This is a vector of tuples, where the first element is the node id and the
/// second element is the Part message (Message containing committment to Bivariate polynomial).
/// * `ack_message_store`: This is a vector of tuples of the form (u16, Ack). The u16 is the index of
/// the node that sent the Ack message. The Ack message is the acknowledgement message that is sent by
/// the node to all the peers after it has received the Part message.
/// * `peer_public_keys`: A map of the public keys of all the peers in the group.
/// * `public_key_set`: Distributed Public Key for Committee of MasterNodes and an associated set of public key shares.
/// * `secret_key_share`: This is the secret key share that we will use to generate the public key set.
/// * `sync_key_gen`: This is the SyncKeyGen object that is used to generate the public and private
/// keys.
/// * `random_number_gen`: This is a random number generator that we'll use to generate random numbers.
pub struct DkgState {
    pub part_message_store: HashMap<u16, Part>,

    pub ack_message_store: HashMap<(u16,u16), Ack>,

    pub peer_public_keys: BTreeMap<u16, PublicKey>,

    pub public_key_set: Option<PublicKeySet>,

    pub secret_key_share: Option<SecretKeyShare>,

    pub sync_key_gen: Option<SyncKeyGen<u16>>,

    pub random_number_gen: Option<OsRng>,

    pub secret_key:SecretKey
}

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
    #[error("Invalid Key {0}  Value {1}")]
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
    PartMessageGenerated(u16,Part),
    PartMessageAcknowledged,
    AllAcksHandled,
}

#[macro_export]
macro_rules! is_enum_variant {
    ($v:expr, $p:pat) => (
        if let $p = $v { true } else { false }
    );
}