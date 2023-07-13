pub mod config;
pub mod dkg;
pub mod engine;
pub mod result;
pub mod test_utils;

pub use config::*;
pub use dkg::*;
pub use engine::*;
pub use result::*;

#[deprecated]
pub mod types {
    pub use super::{config, engine::*, result::*};
}

// #[cfg(test)]
// mod tests {
//     use std::{borrow::BorrowMut, collections::HashMap};
//
//     use hbbft::sync_key_gen::Ack;
//     use primitives::NodeType;
//     use vrrb_core::is_enum_variant;
//
//     use crate::DkgGenerator;
//     use crate::{
//         test_utils::generate_dkg_engines,
//         types::{DkgEngine, DkgError},
//         DkgResult,
//     };
//
//     #[tokio::test]
//     #[ignore]
//     async fn
// failed_to_generate_part_committment_message_since_only_master_node_allowed()
// {         let mut dkg_engines = generate_dkg_engines(4,
// NodeType::Miner).await;         let dkg_engine =
// dkg_engines.get_mut(0).unwrap();         let result =
// dkg_engine.generate_sync_keygen_instance(1);         assert!(result.
// is_err());         assert!(is_enum_variant!(result, Err(DkgError::InvalidNode
// { .. })));     }
//
//     #[tokio::test]
//     #[ignore]
//     async fn generate_part_committment_message() {
//         let mut dkg_engines = generate_dkg_engines(4,
// NodeType::MasterNode).await;         let dkg_engine =
// dkg_engines.get_mut(0).unwrap();         let part_committement_result =
// dkg_engine.generate_sync_keygen_instance(1);         assert!
// (part_committement_result.is_ok());         assert!(is_enum_variant!(
//             part_committement_result,
//             Ok(DkgResult::PartMessageGenerated { .. })
//         ));
//     }
//
//     #[tokio::test]
//     #[ignore]
//     async fn successfull_acknowledge_part_committment_message() {
//         let mut dkg_engines = generate_dkg_engines(4,
// NodeType::MasterNode).await;         let dkg_engine =
// dkg_engines.get_mut(0).unwrap();         dkg_engine.
// generate_sync_keygen_instance(1).unwrap();         let result =
// dkg_engine.ack_partial_commitment(0);         assert!(result.is_ok());
//         assert!(is_enum_variant!(
//             result,
//             Ok(DkgResult::PartMessageAcknowledged)
//         ));
//     }
//
//     #[tokio::test]
//     #[ignore]
//     async fn failed_to_acknowledge_part_committment_missing_committment() {
//         let mut dkg_engines = generate_dkg_engines(4,
// NodeType::MasterNode).await;         let dkg_engine =
// dkg_engines.get_mut(0).unwrap();         let _ =
// dkg_engine.generate_sync_keygen_instance(1);         let result =
// dkg_engine.ack_partial_commitment(1);         assert!(result.is_err());
//         assert!(is_enum_variant!(
//             result,
//             Err(DkgError::PartMsgMissingForNode { .. })
//         ));
//     }
//
//     #[tokio::test]
//     #[ignore]
//     async fn
// failed_to_acknowledge_part_committment_missing_syncgen_instance() {
//         let mut dkg_engines = generate_dkg_engines(4,
// NodeType::MasterNode).await;         let dkg_engine =
// dkg_engines.get_mut(0).unwrap();         let result =
// dkg_engine.ack_partial_commitment(0);         assert!(result.is_err());
//         assert!(is_enum_variant!(
//             result,
//             Err(DkgError::SyncKeyGenInstanceNotCreated { .. })
//         ));
//     }
//
//     #[tokio::test]
//     #[ignore]
//     async fn successfull_acknowledge_all_acks() {
//         let mut dkg_engines = generate_dkg_engines(4,
// NodeType::MasterNode).await;         let mut dkg_engine_node4 =
// dkg_engines.pop().unwrap();         let mut dkg_engine_node3 =
// dkg_engines.pop().unwrap();         let mut dkg_engine_node2 =
// dkg_engines.pop().unwrap();         let mut dkg_engine_node1 =
// dkg_engines.pop().unwrap();
//
//         let _ = dkg_engine_node1.generate_sync_keygen_instance(1);
//         let _ = dkg_engine_node2.generate_sync_keygen_instance(1);
//         let _ = dkg_engine_node3.generate_sync_keygen_instance(1);
//         let _ = dkg_engine_node4.generate_sync_keygen_instance(1);
//
//         add_part_committment_to_node_dkg_state(
//             dkg_engine_node1.borrow_mut(),
//             dkg_engine_node2.borrow_mut(),
//             1,
//         );
//
//         add_part_committment_to_node_dkg_state(
//             dkg_engine_node1.borrow_mut(),
//             dkg_engine_node3.borrow_mut(),
//             2,
//         );
//
//         add_part_committment_to_node_dkg_state(
//             dkg_engine_node1.borrow_mut(),
//             dkg_engine_node4.borrow_mut(),
//             3,
//         );
//
//         dkg_engine_node1.ack_partial_commitment(0).unwrap();
//         dkg_engine_node1.ack_partial_commitment(1).unwrap();
//         dkg_engine_node1.ack_partial_commitment(2).unwrap();
//         dkg_engine_node1.ack_partial_commitment(3).unwrap();
//
//         let result = dkg_engine_node1.handle_ack_messages().unwrap();
//
//         assert!(is_enum_variant!(result, DkgResult::AllAcksHandled { .. }));
//     }
//
//     #[tokio::test]
//     #[ignore]
//     async fn successful_generations_of_key_sets() {
//         let mut dkg_engines = generate_dkg_engines(4,
// NodeType::MasterNode).await;         let mut dkg_engine_node4 =
// dkg_engines.pop().unwrap();         let mut dkg_engine_node3 =
// dkg_engines.pop().unwrap();         let mut dkg_engine_node2 =
// dkg_engines.pop().unwrap();         let mut dkg_engine_node1 =
// dkg_engines.pop().unwrap();
//
//         let part_committment_node1 =
// dkg_engine_node1.generate_sync_keygen_instance(1).unwrap();         let
// part_committment_node2 =
// dkg_engine_node2.generate_sync_keygen_instance(1).unwrap();         let
// part_committment_node3 =
// dkg_engine_node3.generate_sync_keygen_instance(1).unwrap();         let
// part_committment_node4 =
// dkg_engine_node4.generate_sync_keygen_instance(1).unwrap();
//
//         let part_committment_tuples = vec![
//             part_committment_node1,
//             part_committment_node2,
//             part_committment_node3,
//             part_committment_node4,
//         ];
//
//         for part_commitment in part_committment_tuples.iter() {
//             if let DkgResult::PartMessageGenerated(node_id, part) =
// part_commitment {                 if *node_id != dkg_engine_node1.node_id() {
//                     dkg_engine_node1
//                         .dkg_state
//                         .part_message_store
//                         .insert(*node_idx, part.clone());
//                 }
//
//                 if *node_id != dkg_engine_node2.node_id() {
//                     dkg_engine_node2
//                         .dkg_state
//                         .part_message_store
//                         .insert(*node_idx, part.clone());
//                 }
//
//                 if *node_id != dkg_engine_node3.node_id() {
//                     dkg_engine_node3
//                         .dkg_state
//                         .part_message_store
//                         .insert(*node_idx, part.clone());
//                 }
//
//                 if *node_id != dkg_engine_node4.node_id() {
//                     dkg_engine_node4
//                         .dkg_state
//                         .part_message_store
//                         .insert(*node_id, part.clone());
//                 }
//             }
//         }
//
//         for i in 0..4 {
//             let _ = dkg_engine_node1.ack_partial_commitment(i);
//
//             let _ = dkg_engine_node2.ack_partial_commitment(i);
//
//             let _ = dkg_engine_node3.ack_partial_commitment(i);
//
//             let _ = dkg_engine_node4.ack_partial_commitment(i);
//         }
//
//         let mut new_store: HashMap<(u16, u16), Ack>;
//
//         new_store = dkg_engine_node1
//             .dkg_state
//             .ack_message_store
//             .clone()
//             .into_iter()
//             .chain(dkg_engine_node2.dkg_state.ack_message_store.clone())
//             .collect();
//
//         new_store = new_store
//             .into_iter()
//             .chain(dkg_engine_node3.dkg_state.ack_message_store.clone())
//             .collect();
//
//         new_store = new_store
//             .into_iter()
//             .chain(dkg_engine_node4.dkg_state.ack_message_store.clone())
//             .collect();
//
//         dkg_engine_node1.dkg_state.ack_message_store = new_store.clone();
//         dkg_engine_node2.dkg_state.ack_message_store = new_store.clone();
//         dkg_engine_node3.dkg_state.ack_message_store = new_store.clone();
//         dkg_engine_node4.dkg_state.ack_message_store = new_store;
//
//         for _ in 0..4 {
//             dkg_engine_node1.handle_ack_messages().unwrap();
//             dkg_engine_node2.handle_ack_messages().unwrap();
//             dkg_engine_node3.handle_ack_messages().unwrap();
//             dkg_engine_node4.handle_ack_messages().unwrap();
//         }
//
//         let result = dkg_engine_node1.generate_key_sets();
//         assert!(result.is_ok());
//         assert!(dkg_engine_node1.dkg_state.public_key_set.is_some());
//         assert!(dkg_engine_node1.dkg_state.secret_key_share.is_some());
//     }
//
//     fn add_part_committment_to_node_dkg_state(
//         dkg_engine_node1: &mut DkgEngine,
//         dkg_engine_node2: &mut DkgEngine,
//         node_idx: u16,
//     ) {
//         let part_committment_node2 =
// dkg_engine_node2.generate_sync_keygen_instance(1).unwrap();         match
// part_committment_node2 {             DkgResult::PartMessageGenerated(_, part)
// => {                 dkg_engine_node1
//                     .dkg_state
//                     .part_message_store
//                     .insert(node_idx, part);
//             },
//
//             _ => panic!("Wrong Status"),
//         }
//     }
// }
