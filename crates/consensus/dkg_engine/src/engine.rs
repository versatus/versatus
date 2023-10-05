use std::sync::Arc;

use hbbft::{
    crypto::{PublicKey, PublicKeySet, SecretKey},
    sync_key_gen::{Ack, Part, PartOutcome, SyncKeyGen},
};
use primitives::{NodeId, NodeType, ValidatorPublicKey};
use rand::rngs::OsRng;
use vrrb_config::ThresholdConfig;

use crate::{
    prelude::{DkgGenerator, DkgState, ReceiverId, SenderId},
    DkgError, Result,
};

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
    pub node_id: NodeId,

    pub node_type: NodeType,

    /// For DKG (Can be extended for hierarchical DKG)
    pub threshold_config: vrrb_config::ThresholdConfig,

    pub secret_key: SecretKey,

    /// state information related to dkg process
    pub dkg_state: DkgState,

    /// Harvester Distributed  Group public key
    pub harvester_public_key: Option<PublicKey>,
}

impl Clone for DkgEngine {
    fn clone(&self) -> Self {
        let peer_public_keys = Arc::new(self.dkg_state.peer_public_keys().clone());

        // TODO: fix unwraps
        let mut rng = OsRng::new()
            .map_err(|err| DkgError::Unknown(err.to_string()))
            .unwrap();

        let (sync_key_gen, _) = SyncKeyGen::new(
            self.node_id(),
            self.secret_key.clone(),
            peer_public_keys,
            self.threshold_config().threshold as usize,
            &mut rng,
        )
        .unwrap();

        let mut dkg_state = DkgState::new();

        dkg_state.set_part_message_store(self.dkg_state.part_message_store_owned());
        dkg_state.set_ack_message_store(self.dkg_state.ack_message_store_owned());
        dkg_state.set_peer_public_keys(self.dkg_state.peer_public_keys_owned());
        dkg_state.set_public_key_set(self.dkg_state.public_key_set_owned());
        dkg_state.set_secret_key_share(self.dkg_state.secret_key_share_owned());
        dkg_state.set_sync_key_gen(Some(sync_key_gen));
        dkg_state.set_random_number_gen(self.dkg_state.random_number_gen_owned());

        Self {
            node_id: self.node_id.clone(),
            node_type: self.node_type,
            threshold_config: self.threshold_config(),
            secret_key: self.secret_key.clone(),
            dkg_state,
            harvester_public_key: self.harvester_public_key,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DkgEngineConfig {
    pub node_id: NodeId,
    pub node_type: NodeType,
    pub secret_key: SecretKey,
    pub threshold_config: vrrb_config::ThresholdConfig,
}

impl DkgEngine {
    pub fn new(config: DkgEngineConfig) -> DkgEngine {
        DkgEngine {
            node_id: config.node_id,
            node_type: config.node_type,
            secret_key: config.secret_key,
            threshold_config: config.threshold_config,
            dkg_state: DkgState::default(),
            harvester_public_key: None,
        }
    }

    pub fn add_peer_public_key(&mut self, node_id: NodeId, public_key: PublicKey) {
        self.dkg_state
            .peer_public_keys_mut()
            .insert(node_id, public_key);
    }

    pub fn set_harvester_public_key(&mut self, harvester_public_key: ValidatorPublicKey) {
        self.harvester_public_key = Some(harvester_public_key);
    }

    pub fn get_public_key(&self) -> PublicKey {
        self.secret_key.public_key()
    }

    pub fn node_id(&self) -> NodeId {
        self.node_id.clone()
    }

    /// It clears the state of the DKG. it happens during change of Epoch
    pub fn clear_state(&mut self) {
        self.dkg_state.clear();
    }
}

impl DkgGenerator for DkgEngine {
    /// `generate_partial_commitment` is a function that creates a
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
    fn generate_partial_commitment(&mut self, threshold: usize) -> Result<(Part, NodeId)> {
        // if (self.dkg_state.peer_public_keys().len() as u16) != self.threshold_config.upper_bound {
        //     return Err(DkgError::NotEnoughPeerPublicKeys);
        // }

        let node_id = self.node_id();
        let secret_key = self.secret_key.clone();
        let peer_public_keys = Arc::new(self.dkg_state.peer_public_keys().clone());
        let mut rng = OsRng::new().map_err(|err| DkgError::Unknown(err.to_string()))?;

        let (sync_key_gen, opt_part) = SyncKeyGen::new(
            node_id.clone(),
            secret_key,
            peer_public_keys,
            threshold,
            &mut rng,
        )
        .map_err(|err| {
            DkgError::SyncKeyGenError(format!(
                "Failed to create instance for node {:?}: {err}",
                node_id.clone()
            ))
        })?;

        let part_commitment = opt_part.ok_or(DkgError::PartCommitmentNotGenerated)?;

        self.dkg_state.set_random_number_gen(Some(rng.clone()));
        self.dkg_state
            .part_message_store_mut()
            .insert(node_id.clone(), part_commitment.clone());

        self.dkg_state.set_sync_key_gen(Some(sync_key_gen));

        // part_commitment has to be multicasted to all Farmers/Harvester Peers
        // within the Quorum
        Ok((part_commitment, self.node_id()))
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
    fn ack_partial_commitment(
        &mut self,
        sender_node_id: SenderId,
    ) -> Result<(ReceiverId, SenderId, Ack)> {
        let node_id = self.node_id();

        let ack_message_store = self.dkg_state.ack_message_store_owned();
        let part_message_store = self.dkg_state.part_message_store_owned();

        let mut rng = self
            .dkg_state
            .random_number_gen_owned()
            .ok_or(DkgError::Unknown(
                "failed to get random number generator".into(),
            ))?;

        let node = self
            .dkg_state
            .sync_key_gen_mut()
            .as_mut()
            .ok_or(DkgError::SyncKeyGenInstanceNotCreated)?;

        if ack_message_store.contains_key(&(node_id.clone(), sender_node_id.clone())) {
            return Err(DkgError::PartMsgAlreadyAcknowledged(sender_node_id));
        }

        let part_commitment = part_message_store
            .get(&sender_node_id)
            .ok_or(DkgError::PartMsgMissingForNode(sender_node_id.clone()))?;

        let handed_part_result =
            node.handle_part(&sender_node_id, part_commitment.clone(), &mut rng);

        match handed_part_result {
            Ok(part_outcome) => match part_outcome {
                PartOutcome::Valid(Some(ack)) => {
                    self.dkg_state
                        .ack_message_store_mut()
                        .insert((node_id.clone(), sender_node_id.clone()), ack.clone());

                    Ok((node_id, sender_node_id, ack))
                },
                PartOutcome::Invalid(fault) => Err(DkgError::InvalidPartMessage(fault.to_string())),
                PartOutcome::Valid(None) => Err(DkgError::ObserverNotAllowed),
            },
            Err(err) => Err(DkgError::Unknown(format!(
                "failed to generate handle part commitment: {err}",
            ))),
        }
    }

    /// Handles all Acks messages from ack message store
    ///
    /// Returns:
    ///
    /// a Result type. The Result type is an enum that can be either Ok or Err.
    fn handle_ack_messages(&mut self) -> Result<()> {
        let ack_message_store = self.dkg_state.ack_message_store_owned();

        let mut ack_message_store = ack_message_store
            .into_iter()
            .map(|((receiver_id, sender_id), ack)| (receiver_id, sender_id, ack))
            .collect::<Vec<(ReceiverId, SenderId, Ack)>>();

        ack_message_store.sort_by_key(|entry| entry.0.to_owned());

        let keygen = self
            .dkg_state
            .sync_key_gen_mut()
            .as_mut()
            .ok_or(DkgError::SyncKeyGenInstanceNotCreated)?;

        for (receiver_id, sender_id, ack) in ack_message_store {
            let result = keygen
                .handle_ack(&receiver_id, ack.clone())
                .map_err(|err| {
                    DkgError::InvalidAckMessage(format!("from {sender_id} to {receiver_id}: {err}"))
                })?;

            match result {
                hbbft::sync_key_gen::AckOutcome::Valid => {},
                hbbft::sync_key_gen::AckOutcome::Invalid(fault) => {
                    return Err(DkgError::InvalidAckMessage(format!(
                        "Invalid Ack Outcome for Node {:?},Fault: {:?} ,Idx:{:?}",
                        sender_id,
                        fault,
                        self.node_id()
                    )));
                },
            }
        }

        Ok(())
    }

    ///  Generate the  distributed public key and secreykeyshare for the node in
    /// the Quorum
    fn generate_key_sets(&mut self) -> Result<Option<PublicKeySet>> {
        let keygen = self
            .dkg_state
            .sync_key_gen_mut()
            .as_mut()
            .ok_or(DkgError::SyncKeyGenInstanceNotCreated)?;

        if !keygen.is_ready() {
            return Err(DkgError::NotEnoughPartsCompleted);
        }

        let keys = keygen.generate();

        match keys {
            Ok(key) => {
                let (pks, sks) = (key.0, key.1);
                self.dkg_state.set_public_key_set(Some(pks.clone()));
                self.dkg_state.set_secret_key_share(sks);
                Ok(Some(pks.clone()))
            },
            Err(e) => Err(DkgError::Unknown(format!(
                "{}, Node ID {}, Error: {}",
                String::from("Failed to create `PublicKeySet` and `SecretKeyShare`"),
                self.node_id(),
                e
            ))),
        }
    }

    fn threshold_config(&self) -> ThresholdConfig {
        self.threshold_config.clone()
    }

    fn add_peer_public_key(&mut self, node_id: NodeId, public_key: PublicKey) {
        self.dkg_state.add_peer_public_key(node_id, public_key);
    }
}
