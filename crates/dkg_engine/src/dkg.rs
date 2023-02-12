use std::sync::Arc;

use hbbft::sync_key_gen::{PartOutcome, SyncKeyGen};
use primitives::NodeType;

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
        if let Ok(node_info) = self.node_info.read() {
            if node_info.get_node_type() != NodeType::MasterNode {
                return Err(DkgError::InvalidNode);
            }
            let secret_key = node_info.keypair.validator_kp.0.clone();
            if let Ok(mut rng) = rand::rngs::OsRng::new() {
                let (sync_key_gen, opt_part) = SyncKeyGen::new(
                    node_info.get_node_idx(),
                    secret_key,
                    Arc::new(self.dkg_state.peer_public_keys.clone()),
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
                if let Some(part_committment) = opt_part {
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
                } else {
                    Err(DkgError::PartCommitmentNotGenerated)
                }
            } else {
                Err(DkgError::Unknown(String::from(
                    "Failed to generate random number",
                )))
            }
        } else {
            Err(DkgError::Unknown(String::from(
                "Read Write Lock For Node poisoned",
            )))
        }
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
        if let Some(node) = self.dkg_state.sync_key_gen.as_mut() {
            if let Ok(handling_node) = self.node_info.read() {
                let handling_node_idx = handling_node.get_node_idx();
                if self
                    .dkg_state
                    .ack_message_store
                    .contains_key(&(handling_node_idx, sender_node_idx))
                {
                    return Err(DkgError::PartMsgAlreadyAcknowledge(sender_node_idx));
                }
                let part_commitment = self.dkg_state.part_message_store.get(&sender_node_idx);
                if let Some(part_commitment) = part_commitment {
                    if let Some(rng) = self.dkg_state.random_number_gen.as_mut() {
                        match node
                            .handle_part(&sender_node_idx, part_commitment.clone(), rng)
                            .expect("Failed to handle sender_node_idx")
                        {
                            PartOutcome::Valid(Some(ack)) => {
                                self.dkg_state
                                    .ack_message_store
                                    .insert((handling_node_idx, sender_node_idx), ack);
                                Ok(DkgResult::PartMessageAcknowledged)
                            },
                            PartOutcome::Invalid(fault) => {
                                Err(DkgError::InvalidPartMessage(fault.to_string()))
                            },
                            PartOutcome::Valid(None) => Err(DkgError::ObserverNotAllowed),
                        }
                    } else {
                        Err(DkgError::Unknown(String::from(
                            "Failed to generate random number",
                        )))
                    }
                } else {
                    Err(DkgError::PartMsgMissingForNode(sender_node_idx))
                }
            } else {
                Err(DkgError::Unknown(String::from(
                    "Read Write Lock For Node poisoned",
                )))
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
        if let Ok(handling_node) = self.node_info.read() {
            if let Some(node) = self.dkg_state.sync_key_gen.as_mut() {
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
                                    handling_node.get_node_idx()
                                )));
                            },
                        },
                        Err(_) => {
                            let mut id = sender_id.0.to_string();
                            #[allow(clippy::single_char_add_str)]
                            id.push_str(" ");
                            id.push_str(&sender_id.1.to_string());
                            return Err(DkgError::InvalidAckMessage(id));
                        },
                    }
                }
                Ok(DkgResult::AllAcksHandled)
            } else {
                Err(DkgError::SyncKeyGenInstanceNotCreated)
            }
        } else {
            Err(DkgError::Unknown(String::from(
                "Read Write Lock For Node poisoned",
            )))
        }
    }

    ///  Generate the  distributed public key and secreykeyshare for the node in
    /// the Quorum
    fn generate_key_sets(&mut self) -> Self::DkgStatus {
        if let Ok(node) = self.node_info.read() {
            if let Some(synckey_gen) = self.dkg_state.sync_key_gen.as_ref() {
                if !synckey_gen.is_ready() {
                    return Err(DkgError::NotEnoughPartsCompleted);
                }
                let (pks, sks) = synckey_gen.generate().unwrap_or_else(|_| {
                    panic!(
                        "Failed to create `PublicKeySet` and `SecretKeyShare` for node #{}",
                        node.get_node_idx()
                    )
                });
                self.dkg_state.public_key_set = Some(pks);
                self.dkg_state.secret_key_share = sks;
                Ok(DkgResult::KeySetsGenerated)
            } else {
                Err(DkgError::SyncKeyGenInstanceNotCreated)
            }
        } else {
            Err(DkgError::Unknown(String::from(
                "Read Write Lock For Node poisoned",
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{borrow::BorrowMut, collections::HashMap};

    use hbbft::sync_key_gen::Ack;
    use node::NodeType;
    use primitives::is_enum_variant;

    use super::DkgGenerator;
    use crate::{
        dkg::DkgResult,
        test_utils::generate_dkg_engines,
        types::{DkgEngine, DkgError},
    };

    #[tokio::test]
    #[ignore]
    async fn failed_to_generate_part_committment_message_since_only_master_node_allowed() {
        let mut dkg_engines = generate_dkg_engines(4, NodeType::Miner);
        let mut dkg_engine = dkg_engines.await;
        let dkg_engine = dkg_engine.get_mut(0).unwrap();
        let result = dkg_engine.generate_sync_keygen_instance(1);
        assert_eq!(result.is_err(), true);
        assert!(is_enum_variant!(result, Err(DkgError::InvalidNode { .. })));
    }

    #[tokio::test]
    #[ignore]
    async fn generate_part_committment_message() {
        let dkg_engines = generate_dkg_engines(4, NodeType::MasterNode);
        let mut dkg_engine = dkg_engines.await;
        let dkg_engine = dkg_engine.get_mut(0).unwrap();
        let part_committement_result = dkg_engine.generate_sync_keygen_instance(1);
        assert_eq!(part_committement_result.is_ok(), true);
        assert!(is_enum_variant!(
            part_committement_result,
            Ok(DkgResult::PartMessageGenerated { .. })
        ));
    }

    #[tokio::test]
    #[ignore]
    async fn successfull_acknowledge_part_committment_message() {
        let mut dkg_engines = generate_dkg_engines(4, NodeType::MasterNode);
        let mut dkg_engine = dkg_engines.await;
        let dkg_engine = dkg_engine.get_mut(0).unwrap();
        let _ = dkg_engine.generate_sync_keygen_instance(1);
        let result = dkg_engine.ack_partial_commitment(0);
        assert_eq!(result.is_ok(), true);
        assert!(is_enum_variant!(
            result,
            Ok(DkgResult::PartMessageAcknowledged)
        ));
    }

    #[tokio::test]
    #[ignore]
    async fn failed_to_acknowledge_part_committment_missing_committment() {
        let mut dkg_engines = generate_dkg_engines(4, NodeType::MasterNode);

        let mut dkg_engine = dkg_engines.await;
        let dkg_engine = dkg_engine.get_mut(0).unwrap();
        let _ = dkg_engine.generate_sync_keygen_instance(1);
        let result = dkg_engine.ack_partial_commitment(1);
        assert_eq!(result.is_err(), true);
        assert!(is_enum_variant!(
            result,
            Err(DkgError::PartMsgMissingForNode { .. })
        ));
    }

    #[tokio::test]
    #[ignore]
    async fn failed_to_acknowledge_part_committment_missing_syncgen_instance() {
        let mut dkg_engines = generate_dkg_engines(4, NodeType::MasterNode);
        let mut dkg_engine = dkg_engines.await;
        let dkg_engine = dkg_engine.get_mut(0).unwrap();
        let result = dkg_engine.ack_partial_commitment(0);
        assert_eq!(result.is_err(), true);
        assert!(is_enum_variant!(
            result,
            Err(DkgError::SyncKeyGenInstanceNotCreated { .. })
        ));
    }

    #[tokio::test]
    #[ignore]
    async fn successfull_acknowledge_all_acks() {
        let mut dkg_engines = generate_dkg_engines(4, NodeType::MasterNode);
        let mut dkg_engines = dkg_engines.await;

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

    #[tokio::test]
    #[ignore]
    async fn successful_generations_of_key_sets() {
        let mut dkg_engines = generate_dkg_engines(4, NodeType::MasterNode);
        let mut dkg_engines = dkg_engines.await;

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

        /*
                println!("Node 1{:?}", dkg_engine_node1.dkg_state.part_message_store);

                println!("Node 2{:?}", dkg_engine_node2.dkg_state.part_message_store);

                println!("Node 3{:?}", dkg_engine_node3.dkg_state.part_message_store);

                println!("Node 4{:?}", dkg_engine_node4.dkg_state.part_message_store);
        */
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

        /*
                println!("Node 1{:?}", dkg_engine_node1.dkg_state.ack_message_store);

                println!("Node 2{:?}", dkg_engine_node2.dkg_state.ack_message_store);

                println!("Node 3{:?}", dkg_engine_node3.dkg_state.ack_message_store);

                println!("Node 4{:?}", dkg_engine_node4.dkg_state.ack_message_store);
        */
        let mut new_store: HashMap<(u16, u16), Ack>;
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
            dkg_engine_node1.handle_ack_messages().unwrap();
            dkg_engine_node2.handle_ack_messages().unwrap();
            dkg_engine_node3.handle_ack_messages().unwrap();
            dkg_engine_node4.handle_ack_messages().unwrap();
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
