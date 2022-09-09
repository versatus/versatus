use hbbft::{
    crypto::{serde_impl::SerdeSecret, SecretKey},
    sync_key_gen::{PartOutcome, SyncKeyGen},
};
use node::node::NodeType;

use crate::types::{DkgEngine, DkgError, DkgResult};

/// This is a trait that is implemented by the `DkgEngine` struct. It contains
/// the functions that are required to run the DKG protocol.
pub trait DkgGenerator {
    type DkgStatus;

    fn generate_sync_keygen_instance(&mut self, threshold: usize) -> Self::DkgStatus;

    fn ack_partial_commitment(&mut self, node_idx: u16) -> Self::DkgStatus; //PartOutCome to be sent to channel for broadcasting it to other peers

    fn handle_ack_messages(&mut self) -> Self::DkgStatus; //Handle all ACK Messages from all other k-1 MasterNodes

    fn generate_key_sets(&mut self) -> Self::DkgStatus;
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
            return Err(DkgError::NotEnoughPeerPublicKeys);
        }
        let node_info = self.node_info.read().unwrap();
        if node_info.get_node_type() != NodeType::MasterNode {
            return Err(DkgError::InvalidNode);
        }
        // TODO code to import secret key from node info to be added
        let secret_key_encoded = self.node_info.read().unwrap().secret_key.clone();

        //This need to be moved to either primitive(Generics) module or Node module

        let secret_key =
            bincode::deserialize::<SerdeSecret<SecretKey>>(secret_key_encoded.as_slice());
        if secret_key.is_err() {
            return Err(DkgError::Unknown(format!(
                "Failed to deserialize the secret key for node {:?}",
                node_info.get_node_idx()
            )));
        }
        let mut rng = rand::rngs::OsRng::new().unwrap();
        let (sync_key_gen, opt_part) = SyncKeyGen::new(
            node_info.get_node_idx(),
            secret_key.unwrap().inner().clone(),
            self.dkg_state.peer_public_keys.clone(),
            threshold,
            &mut rng,
        )
        .unwrap_or_else(|_| {
            panic!(
                "Error :{:?}",
                DkgError::SyncKeyGenError(format!(
                    "Failed to create `SyncKeyGen` instance for node #{:?}",
                    node_info.get_node_idx()
                ))
            )
        });
        let part_committment = opt_part.unwrap();
        self.dkg_state.random_number_gen = Some(rng.clone());
        self.dkg_state
            .part_message_store
            .insert(node_info.get_node_idx(), part_committment.clone());
        self.dkg_state.sync_key_gen = Some(sync_key_gen);

        //part_commitment has to be multicasted to all LLMQ Peers
        Ok(DkgResult::PartMessageGenerated(
            node_info.get_node_idx(),
            part_committment,
        ))
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
    fn ack_partial_commitment(&mut self, sender_node_idx: u16) -> Self::DkgStatus {
        let node = self.dkg_state.sync_key_gen.as_mut();
        if node.is_none() {
            return Err(DkgError::SyncKeyGenInstanceNotCreated);
        }
        let handling_node_idx = self.node_info.read().unwrap().get_node_idx();
        if self
            .dkg_state
            .ack_message_store
            .contains_key(&(handling_node_idx, sender_node_idx))
        {
            return Err(DkgError::PartMsgAlreadyAcknowledge(sender_node_idx));
        }
        let node = node.unwrap();
        let part_committment = self.dkg_state.part_message_store.get(&sender_node_idx);
        if part_committment.is_some() {
            let part_committment = part_committment.unwrap();
            let rng = self.dkg_state.random_number_gen.as_mut().unwrap();
            match node
                .handle_part(&sender_node_idx, part_committment.clone(), rng)
                .expect("Failed to handle sender_node_idx")
            {
                PartOutcome::Valid(Some(ack)) => {
                    self.dkg_state
                        .ack_message_store
                        .insert((handling_node_idx, sender_node_idx), ack);
                    return Ok(DkgResult::PartMessageAcknowledged);
                },
                PartOutcome::Invalid(fault) => {
                    return Err(DkgError::InvalidPartMessage(fault.to_string()));
                },
                PartOutcome::Valid(None) => {
                    return Err(DkgError::ObserverNotAllowed);
                },
            }
        } else {
            return Err(DkgError::PartMsgMissingForNode(sender_node_idx));
        }
    }

    /// Handles all Acks messages from ack message store
    ///
    /// Returns:
    ///
    /// a Result type. The Result type is an enum that can be either Ok or Err.
    fn handle_ack_messages(&mut self) -> Self::DkgStatus {
        /*
        if self.dkg_state.ack_message_store.len() as u16 != self.threshold_config.upper_bound {
            return Err(DkgError::NotEnoughAckMsgsReceived);
        }
        */
        let node = self.dkg_state.sync_key_gen.as_mut().unwrap();
        //    let node_id=self.node_info.read().unwrap().get_node_idx();
        for (sender_id, ack) in &self.dkg_state.ack_message_store {
            let result = node.handle_ack(&sender_id.0, ack.clone());
            if result.is_err() {
                let mut id = sender_id.0.to_string();
                id.push_str(&" ".to_string());
                id.push_str(&sender_id.1.to_string());
                return Err(DkgError::InvalidAckMessage(id));
            } else {
                match result.unwrap() {
                    hbbft::sync_key_gen::AckOutcome::Valid => {},
                    hbbft::sync_key_gen::AckOutcome::Invalid(fault) => {
                        println!(
                            "Sender ID {:?},Invalid {:?}, Node ID :{:?}",
                            sender_id,
                            fault,
                            self.node_info.read().unwrap().get_node_idx()
                        )
                    },
                }
            }
        }
        Ok(DkgResult::AllAcksHandled)
    }

    ///  Generate the  distributed public key and secreykeyshare for the node in
    /// the Quorum
    fn generate_key_sets(&mut self) -> Self::DkgStatus {
        let node_idx = self.node_info.read().unwrap().get_node_idx();
        let synckey_gen = self.dkg_state.sync_key_gen.as_ref();
        if synckey_gen.is_none() {
            return Err(DkgError::SyncKeyGenInstanceNotCreated);
        }
        let synckey_gen = synckey_gen.unwrap();
        // This is a check to see if the threshold+1 part committments are verified and
        // acknowledged for generation of DKG .
        if !synckey_gen.is_ready() {
            return Err(DkgError::NotEnoughPartsCompleted);
        }

        let (pks, sks) = synckey_gen.generate().unwrap_or_else(|_| {
            panic!(
                "Failed to create `PublicKeySet` and `SecretKeyShare` for node #{}",
                node_idx
            )
        });
        self.dkg_state.public_key_set = Some(pks);
        self.dkg_state.secret_key_share = sks;
        Ok(DkgResult::KeySetsGenerated)
    }
}

#[cfg(test)]
mod tests {
    use std::{borrow::BorrowMut, collections::HashMap};

    use hbbft::sync_key_gen::Ack;
    use node::node::NodeType;
    use primitives::is_enum_variant;

    // use super::*;
    use super::DkgGenerator;
    use crate::{
        dkg::DkgResult,
        test_utils::generate_dkg_engines,
        types::{DkgEngine, DkgError},
    };

    #[test]
    fn failed_to_generate_part_committment_message_since_only_master_node_allowed() {
        let mut dkg_engines = generate_dkg_engines(4, NodeType::Miner);
        let dkg_engine = dkg_engines.get_mut(0).unwrap();
        let result = dkg_engine.generate_sync_keygen_instance(1);
        assert_eq!(result.is_err(), true);
        assert!(is_enum_variant!(result, Err(DkgError::InvalidNode { .. })));
    }

    #[test]
    fn generate_part_committment_message() {
        let mut dkg_engines = generate_dkg_engines(4, NodeType::MasterNode);
        let dkg_engine = dkg_engines.get_mut(0).unwrap();
        let part_committement_result = dkg_engine.generate_sync_keygen_instance(1);
        assert_eq!(part_committement_result.is_ok(), true);
        assert!(is_enum_variant!(
            part_committement_result,
            Ok(DkgResult::PartMessageGenerated { .. })
        ));
    }

    #[test]
    fn successfull_acknowledge_part_committment_message() {
        let mut dkg_engines = generate_dkg_engines(4, NodeType::MasterNode);
        let dkg_engine = dkg_engines.get_mut(0).unwrap();
        let _ = dkg_engine.generate_sync_keygen_instance(1);
        let result = dkg_engine.ack_partial_commitment(0);
        assert_eq!(result.is_ok(), true);
        assert!(is_enum_variant!(
            result,
            Ok(DkgResult::PartMessageAcknowledged)
        ));
    }

    #[test]
    fn failed_to_acknowledge_part_committment_missing_committment() {
        let mut dkg_engines = generate_dkg_engines(4, NodeType::MasterNode);
        let dkg_engine = dkg_engines.get_mut(0).unwrap();
        let _ = dkg_engine.generate_sync_keygen_instance(1);
        let result = dkg_engine.ack_partial_commitment(1);
        assert_eq!(result.is_err(), true);
        assert!(is_enum_variant!(
            result,
            Err(DkgError::PartMsgMissingForNode { .. })
        ));
    }

    #[test]
    fn failed_to_acknowledge_part_committment_missing_syncgen_instance() {
        let mut dkg_engines = generate_dkg_engines(4, NodeType::MasterNode);
        let dkg_engine = dkg_engines.get_mut(0).unwrap();
        let result = dkg_engine.ack_partial_commitment(0);
        assert_eq!(result.is_err(), true);
        assert!(is_enum_variant!(
            result,
            Err(DkgError::SyncKeyGenInstanceNotCreated { .. })
        ));
    }

    #[test]
    fn successfull_acknowledge_all_acks() {
        let mut dkg_engines = generate_dkg_engines(4, NodeType::MasterNode);
        let mut dkg_engine_node4 = dkg_engines.pop().unwrap();
        let mut dkg_engine_node3 = dkg_engines.pop().unwrap();
        let mut dkg_engine_node2 = dkg_engines.pop().unwrap();
        let mut dkg_engine_node1 = dkg_engines.pop().unwrap();

        let _ = dkg_engine_node1.generate_sync_keygen_instance(1);
        let _ = dkg_engine_node2.generate_sync_keygen_instance(1);
        let _ = dkg_engine_node3.generate_sync_keygen_instance(1);
        let _ = dkg_engine_node4.generate_sync_keygen_instance(1);

        add_part_committment_to_node_dkg_state(
            dkg_engine_node1.borrow_mut(),
            dkg_engine_node2.borrow_mut(),
            1,
        );

        add_part_committment_to_node_dkg_state(
            dkg_engine_node1.borrow_mut(),
            dkg_engine_node3.borrow_mut(),
            2,
        );

        add_part_committment_to_node_dkg_state(
            dkg_engine_node1.borrow_mut(),
            dkg_engine_node4.borrow_mut(),
            3,
        );

        let _ = dkg_engine_node1.ack_partial_commitment(0);
        let _ = dkg_engine_node1.ack_partial_commitment(1);
        let _ = dkg_engine_node1.ack_partial_commitment(2);
        let _ = dkg_engine_node1.ack_partial_commitment(3);

        let result = dkg_engine_node1.handle_ack_messages();

        assert_eq!(result.is_ok(), true);
        assert!(is_enum_variant!(
            result,
            Ok(DkgResult::AllAcksHandled { .. })
        ));
    }

    #[test]
    fn successfull_generations_of_key_sets() {
        let mut dkg_engines = generate_dkg_engines(4, NodeType::MasterNode);
        let mut dkg_engine_node4 = dkg_engines.pop().unwrap();
        let mut dkg_engine_node3 = dkg_engines.pop().unwrap();
        let mut dkg_engine_node2 = dkg_engines.pop().unwrap();
        let mut dkg_engine_node1 = dkg_engines.pop().unwrap();

        let part_committment_node1 = dkg_engine_node1.generate_sync_keygen_instance(1).unwrap();
        let part_committment_node2 = dkg_engine_node2.generate_sync_keygen_instance(1).unwrap();
        let part_committment_node3 = dkg_engine_node3.generate_sync_keygen_instance(1).unwrap();
        let part_committment_node4 = dkg_engine_node4.generate_sync_keygen_instance(1).unwrap();

        let part_committment_tuples = vec![
            part_committment_node1,
            part_committment_node2,
            part_committment_node3,
            part_committment_node4,
        ];

        for part_commitment in part_committment_tuples.iter() {
            if let DkgResult::PartMessageGenerated(node_idx, part) = part_commitment {
                if *node_idx as u16 != dkg_engine_node1.node_info.read().unwrap().get_node_idx() {
                    dkg_engine_node1
                        .dkg_state
                        .part_message_store
                        .insert(*node_idx as u16, part.clone());
                }
                if *node_idx as u16 != dkg_engine_node2.node_info.read().unwrap().get_node_idx() {
                    dkg_engine_node2
                        .dkg_state
                        .part_message_store
                        .insert(*node_idx as u16, part.clone());
                }
                if *node_idx as u16 != dkg_engine_node3.node_info.read().unwrap().get_node_idx() {
                    dkg_engine_node3
                        .dkg_state
                        .part_message_store
                        .insert(*node_idx as u16, part.clone());
                }
                if *node_idx as u16 != dkg_engine_node4.node_info.read().unwrap().get_node_idx() {
                    dkg_engine_node4
                        .dkg_state
                        .part_message_store
                        .insert(*node_idx as u16, part.clone());
                }
            }
        }

        println!("Node 1{:?}", dkg_engine_node1.dkg_state.part_message_store);

        println!("Node 2{:?}", dkg_engine_node2.dkg_state.part_message_store);

        println!("Node 3{:?}", dkg_engine_node3.dkg_state.part_message_store);

        println!("Node 4{:?}", dkg_engine_node4.dkg_state.part_message_store);

        // let dkg_engine_node1_acks=vec![];
        for i in 0..4 {
            let _ = dkg_engine_node1.ack_partial_commitment(i);
            //println!("Node 1 Ack Part Committment status {:?}", s);

            let _ = dkg_engine_node2.ack_partial_commitment(i);
            //  println!("Node 2 Ack Part Committment status {:?}", s);

            let _ = dkg_engine_node3.ack_partial_commitment(i);
            //println!("Node 3 Ack Part Committment status{:?}", s);

            let _ = dkg_engine_node4.ack_partial_commitment(i);
            // println!("Node 4 Ack Part Committment status {:?}", s);
        }

        println!("Node 1{:?}", dkg_engine_node1.dkg_state.ack_message_store);

        println!("Node 2{:?}", dkg_engine_node2.dkg_state.ack_message_store);

        println!("Node 3{:?}", dkg_engine_node3.dkg_state.ack_message_store);

        println!("Node 4{:?}", dkg_engine_node4.dkg_state.ack_message_store);

        let mut new_store: HashMap<(u16, u16), Ack> = HashMap::new();
        new_store = dkg_engine_node1
            .dkg_state
            .ack_message_store
            .clone()
            .into_iter()
            .chain(dkg_engine_node2.dkg_state.ack_message_store.clone())
            .collect();
        new_store = new_store
            .into_iter()
            .chain(dkg_engine_node3.dkg_state.ack_message_store.clone())
            .collect();
        new_store = new_store
            .into_iter()
            .chain(dkg_engine_node4.dkg_state.ack_message_store.clone())
            .collect();

        dkg_engine_node1.dkg_state.ack_message_store = new_store.clone();
        dkg_engine_node2.dkg_state.ack_message_store = new_store.clone();
        dkg_engine_node3.dkg_state.ack_message_store = new_store.clone();
        dkg_engine_node4.dkg_state.ack_message_store = new_store;

        for _ in 0..4 {
            let _ = dkg_engine_node1.handle_ack_messages();
            // println!("Status {:?}", s);
            let _ = dkg_engine_node2.handle_ack_messages();
            // println!("Status {:?}", s);

            let _ = dkg_engine_node3.handle_ack_messages();
            //            println!("Status {:?}", s);

            let _ = dkg_engine_node4.handle_ack_messages();
            //println!("Status {:?}", s);
        }

        let result = dkg_engine_node1.generate_key_sets();
        assert_eq!(result.is_ok(), true);
        assert_eq!(dkg_engine_node1.dkg_state.public_key_set.is_some(), true);
        assert_eq!(dkg_engine_node1.dkg_state.secret_key_share.is_some(), true);
    }

    fn add_part_committment_to_node_dkg_state(
        dkg_engine_node1: &mut DkgEngine,
        dkg_engine_node2: &mut DkgEngine,
        node_idx: u16,
    ) {
        let part_committment_node2 = dkg_engine_node2.generate_sync_keygen_instance(1).unwrap();
        match part_committment_node2 {
            DkgResult::PartMessageGenerated(_, part) => {
                dkg_engine_node1
                    .dkg_state
                    .part_message_store
                    .insert(node_idx, part);
            },

            _ => panic!("Wrong Status"),
        }
    }
}
