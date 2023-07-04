use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use hbbft::{
    crypto::{PublicKey, PublicKeySet, SecretKey, SecretKeyShare},
    sync_key_gen::{Ack, Part, PartOutcome, SyncKeyGen},
};
use primitives::{NodeId, NodeIdx, NodeType};
use rand::rngs::OsRng;

use crate::{config::ThresholdConfig, DkgError, DkgGenerator, DkgResult};

pub type SenderId = NodeId;

#[derive(Debug)]
pub struct DkgState {
    pub part_message_store: HashMap<NodeId, Part>,
    pub ack_message_store: HashMap<(NodeId, SenderId), Ack>,
    pub peer_public_keys: BTreeMap<NodeId, PublicKey>,
    pub public_key_set: Option<PublicKeySet>,
    pub secret_key_share: Option<SecretKeyShare>,
    pub sync_key_generator: Option<SyncKeyGen<NodeId>>,
    pub random_number_gen: Option<OsRng>,
}

/// `DkgEngine` is a struct that holds entry point for initiating DKG
///
/// Properties:
///
/// * `node_info`: This is a struct that contains information about the node. It
///   contains the node type
/// (leader or follower) and the node index.
/// * `threshold_config`: This is the configuration for the threshold scheme. It
///   contains the number of
/// nodes in the network, the threshold, and the number of nodes that are
/// required to be online for the threshold scheme to work.
/// * `dkg_state`: This is the state of the DKG protocol. It is a struct that
///   contains the following
/// properties:
#[derive(Debug)]
pub struct DkgEngine {
    pub(crate) node_id: NodeId,

    pub(crate) node_type: NodeType,

    /// For DKG (Can be extended for heirarchical DKG)
    pub(crate) threshold_config: ThresholdConfig,

    pub(crate) secret_key: SecretKey,

    /// State information related to dkg process
    pub(crate) dkg_state: DkgState,

    /// Harvester Distributed  Group public key
    pub(crate) harvester_public_key: Option<PublicKey>,
}

impl DkgEngine {
    pub fn new(
        node_id: NodeId,
        node_idx: NodeIdx,
        node_type: NodeType,
        secret_key: SecretKey,
        threshold_config: ThresholdConfig,
    ) -> DkgEngine {
        DkgEngine {
            node_id,
            node_type,
            secret_key,
            threshold_config,
            dkg_state: DkgState {
                part_message_store: HashMap::new(),
                ack_message_store: HashMap::new(),
                peer_public_keys: BTreeMap::new(),
                public_key_set: None,
                secret_key_share: None,
                sync_key_generator: None,
                random_number_gen: None,
            },
            harvester_public_key: None,
        }
    }

    pub fn add_peer_public_key(&mut self, node_id: NodeId, public_key: PublicKey) {
        self.dkg_state.peer_public_keys.insert(node_id, public_key);
    }

    pub fn get_public_key(&self) -> PublicKey {
        self.secret_key.public_key()
    }

    pub fn node_id(&self) -> NodeId {
        self.node_id.clone()
    }

    pub fn node_type(&self) -> NodeType {
        self.node_type
    }

    pub fn threshold_config(&self) -> ThresholdConfig {
        todo!()
    }

    pub fn harvester_public_key(&self) -> Option<PublicKey> {
        todo!()
    }

    /// It clears the state of the DKG. it happens during change of Epoch
    pub fn clear_state(&mut self) {
        self.dkg_state.part_message_store.clear();
        self.dkg_state.ack_message_store.clear();
        self.dkg_state.peer_public_keys.clear();
        self.dkg_state.sync_key_generator = None;
        self.dkg_state.random_number_gen = None;
        self.dkg_state.public_key_set = None;
        self.dkg_state.secret_key_share = None;
    }
}

impl DkgGenerator for DkgEngine {
    type DkgStatus = Result<DkgResult, DkgError>;

    /// `generate_sync_keygen_instance` is a function that creates a
    /// `SyncKeyGen` instance for the current node and returns the `Part`
    /// message that needs to be multicasted to all LLMQ peers
    ///
    /// Arguments:
    ///
    /// * `threshold`: The minimum number of nodes that must participate in the
    ///   DKG process.
    ///
    /// Returns:
    ///
    /// The part_commitment is being returned.
    fn generate_sync_keygen_instance(&mut self, threshold: usize) -> Self::DkgStatus {
        if self.dkg_state.peer_public_keys.len() as u16 != self.threshold_config.upper_bound {
            return Err(DkgError::NotEnoughPeerPublicKeys(
                self.dkg_state.peer_public_keys.len(),
                self.threshold_config.upper_bound as usize,
            ));
        }

        if self.node_type != NodeType::MasterNode {
            return Err(DkgError::InvalidNode);
        }

        let secret_key = self.secret_key.clone();

        let mut rng = OsRng::new().map_err(|err| DkgError::Unknown(err.to_string()))?;

        let sync_keygen = SyncKeyGen::new(
            self.node_id(),
            secret_key,
            Arc::new(self.dkg_state.peer_public_keys.clone()),
            threshold,
            &mut rng,
        )
        .map_err(|err| DkgError::Unknown(err.to_string()))?;

        let (sync_key_gen, opt_part) = sync_keygen;

        // match sync_key_gen_instance_result {
        // Ok((sync_key_gen, opt_part)) => {
        if let Some(part_committment) = opt_part {
            self.dkg_state.random_number_gen = Some(rng.clone());
            self.dkg_state
                .part_message_store
                .insert(self.node_id(), part_committment.clone());

            self.dkg_state.sync_key_generator = Some(sync_key_gen);

            //part_commitment has to be multicasted to all Farmers/Harvester Peers
            // within the Quorum

            Ok(DkgResult::PartMessageGenerated(
                self.node_id(),
                part_committment,
            ))
        } else {
            Err(DkgError::PartCommitmentNotGenerated)
        }
        // },

        // Err(err) => Err(DkgError::SyncKeyGenError(format!(
        //     "Failed to create `SyncKeyGen` instance for node #{:?}: {err}",
        //     self.node_id.clone()
        // ))),
        // }
    }

    /// The function `ack_partial_commitment` is used to acknowledge that
    /// current node has verified validator part message
    ///
    /// Arguments:
    ///
    /// * `node_idx`: The index of the node that sent the partial commitment.
    ///
    /// Returns:
    ///
    /// a `Result` type. The `Result` type is an enum with two variants:
    /// `DkgResult` and `Err`.
    fn ack_partial_commitment(&mut self, sender_node_id: NodeId) -> Self::DkgStatus {
        let own_node_id = self.node_id();

        if let Some(node) = self.dkg_state.sync_key_generator.as_mut() {
            if self
                .dkg_state
                .ack_message_store
                .contains_key(&(own_node_id, sender_node_id.clone()))
            {
                return Err(DkgError::PartMsgAlreadyAcknowledge(sender_node_id.clone()));
            }

            let part_commitment = self.dkg_state.part_message_store.get(&sender_node_id);

            if let Some(part_commitment) = part_commitment {
                if let Some(rng) = self.dkg_state.random_number_gen.as_mut() {
                    let handed_part_result =
                        node.handle_part(&sender_node_id, part_commitment.clone(), rng);

                    match handed_part_result {
                        Ok(part_outcome) => match part_outcome {
                            PartOutcome::Valid(Some(ack)) => {
                                self.dkg_state
                                    .ack_message_store
                                    .insert((self.node_id(), sender_node_id), ack);

                                Ok(DkgResult::PartMessageAcknowledged)
                            },
                            PartOutcome::Invalid(fault) => {
                                Err(DkgError::InvalidPartMessage(fault.to_string()))
                            },
                            PartOutcome::Valid(None) => Err(DkgError::ObserverNotAllowed),
                        },
                        Err(e) => Err(DkgError::Unknown(format!(
                            "Failed to generate handle part commitment , error details {:}",
                            e,
                        ))),
                    }
                } else {
                    Err(DkgError::Unknown(String::from(
                        "Failed to generate random number",
                    )))
                }
            } else {
                Err(DkgError::PartMsgMissingForNode(sender_node_id))
            }
        } else {
            Err(DkgError::SyncKeyGenInstanceNotCreated)
        }
    }

    /// Handles all Acks messages from ack message store
    ///
    /// Returns:
    ///
    /// a Result type. The Result type is an enum that can be either Ok or Err.
    fn handle_ack_messages(&mut self) -> Self::DkgStatus {
        if let Some(node) = self.dkg_state.sync_key_generator.as_mut() {
            for (sender_id, ack) in &self.dkg_state.ack_message_store {
                let result = node.handle_ack(&sender_id.0, ack.clone());
                match result {
                    Ok(result) => match result {
                        hbbft::sync_key_gen::AckOutcome::Valid => {},
                        hbbft::sync_key_gen::AckOutcome::Invalid(fault) => {
                            return Err(DkgError::InvalidAckMessage(format!(
                                "Invalid Ack Outcome for Node {:?},Fault: {:?} ,Idx:{:?}",
                                sender_id,
                                fault,
                                self.node_id.clone()
                            )));
                        },
                    },
                    Err(_) => {
                        return Err(DkgError::InvalidAckMessage(format!(
                            "{} {}",
                            sender_id.0,
                            &sender_id.1.to_string()
                        )));
                    },
                }
            }
            Ok(DkgResult::AllAcksHandled)
        } else {
            Err(DkgError::SyncKeyGenInstanceNotCreated)
        }
    }

    ///  Generate the distributed public key and secret key share for the node
    /// in the Quorum
    fn generate_key_sets(&mut self) -> Self::DkgStatus {
        if let Some(synckey_gen) = self.dkg_state.sync_key_generator.as_ref() {
            if !synckey_gen.is_ready() {
                return Err(DkgError::NotEnoughPartsCompleted);
            }

            let (public_keyset, share) = synckey_gen.generate().map_err(|err| {
                DkgError::Unknown(format!(
                    "{}, Node ID {}, Error: {}",
                    String::from("Failed to create `PublicKeySet` and `SecretKeyShare`"),
                    self.node_id.clone(),
                    err
                ))
            })?;

            self.dkg_state.public_key_set = Some(public_keyset);
            self.dkg_state.secret_key_share = share;

            Ok(DkgResult::KeySetsGenerated)
        } else {
            Err(DkgError::SyncKeyGenInstanceNotCreated)
        }
    }
}
