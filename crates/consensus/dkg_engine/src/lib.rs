// pub mod dkg;
// pub mod dkg_state;
// pub mod engine;
// pub mod result;
// pub mod test_utils;

// pub use crate::result::*;

// pub mod prelude {
//     pub use crate::dkg::*;
//     pub use crate::dkg_state::*;
//     pub use crate::engine::*;
// }

// #[cfg(test)]
// mod tests {
//     use std::{borrow::BorrowMut, collections::HashMap};

//     use hbbft::sync_key_gen::Ack;
//     use primitives::{NodeId, NodeType};
//     use vrrb_core::is_enum_variant;

//     use crate::dkg::DkgGenerator;
//     use crate::{prelude::*, result::DkgError, test_utils::generate_dkg_engines};

//     #[tokio::test]
//     #[ignore]
//     async fn failed_to_generate_part_commitment_message_since_only_master_node_allowed() {
//         let mut dkg_engines = generate_dkg_engines(4, NodeType::Miner).await;
//         let dkg_engine = dkg_engines.get_mut(0).unwrap();
//         let result = dkg_engine.generate_partial_commitment(1);

//         assert!(result.is_err());
//         assert!(is_enum_variant!(result, Err(DkgError::InvalidNode { .. })));
//     }

//     #[tokio::test]
//     async fn generate_part_commitment_message() {
//         let mut dkg_engines = generate_dkg_engines(4, NodeType::MasterNode).await;
//         let dkg_engine = dkg_engines.get_mut(0).unwrap();
//         let (part, _) = dkg_engine.generate_partial_commitment(1).unwrap();
//     }

//     #[tokio::test]
//     async fn successfull_acknowledge_part_commitment_message() {
//         let mut dkg_engines = generate_dkg_engines(4, NodeType::MasterNode).await;
//         let dkg_engine = dkg_engines.get_mut(0).unwrap();
//         let _ = dkg_engine.generate_partial_commitment(1);
//         dkg_engine
//             .ack_partial_commitment(String::from("node-0"))
//             .unwrap();
//     }

//     #[tokio::test]
//     async fn failed_to_acknowledge_part_commitment_missing_commitment() {
//         let mut dkg_engines = generate_dkg_engines(4, NodeType::MasterNode).await;
//         let dkg_engine = dkg_engines.get_mut(0).unwrap();
//         let _ = dkg_engine.generate_partial_commitment(1).unwrap();
//         let result = dkg_engine.ack_partial_commitment(String::from("node-1"));
//         assert!(result.is_err());
//         assert!(is_enum_variant!(
//             result,
//             Err(DkgError::PartMsgMissingForNode { .. })
//         ));
//     }

//     #[tokio::test]
//     async fn failed_to_acknowledge_part_commitment_missing_syncgen_instance() {
//         let mut dkg_engines = generate_dkg_engines(4, NodeType::MasterNode).await;
//         let dkg_engine = dkg_engines.get_mut(0).unwrap();
//         let result = dkg_engine.ack_partial_commitment(String::from("node-0"));

//         assert!(result.is_err());
//         assert!(is_enum_variant!(result, Err(DkgError::Unknown { .. })));
//     }

//     #[tokio::test]
//     async fn successfull_acknowledge_all_acks() {
//         let mut dkg_engines = generate_dkg_engines(4, NodeType::MasterNode).await;
//         let mut dkg_engine_node4 = dkg_engines.pop().unwrap();
//         let mut dkg_engine_node3 = dkg_engines.pop().unwrap();
//         let mut dkg_engine_node2 = dkg_engines.pop().unwrap();
//         let mut dkg_engine_node1 = dkg_engines.pop().unwrap();

//         let (_, id_1) = dkg_engine_node1.generate_partial_commitment(1).unwrap();
//         let (_, id_2) = dkg_engine_node2.generate_partial_commitment(1).unwrap();
//         let (_, id_3) = dkg_engine_node3.generate_partial_commitment(1).unwrap();
//         let (_, id_4) = dkg_engine_node4.generate_partial_commitment(1).unwrap();

//         add_part_commitment_to_node_dkg_state(
//             dkg_engine_node1.borrow_mut(),
//             dkg_engine_node2.borrow_mut(),
//             id_1,
//         );

//         add_part_commitment_to_node_dkg_state(
//             dkg_engine_node1.borrow_mut(),
//             dkg_engine_node3.borrow_mut(),
//             id_2,
//         );

//         add_part_commitment_to_node_dkg_state(
//             dkg_engine_node1.borrow_mut(),
//             dkg_engine_node4.borrow_mut(),
//             id_3,
//         );

//         let _ = dkg_engine_node1.ack_partial_commitment(String::from("node-0"));
//         let _ = dkg_engine_node1.ack_partial_commitment(String::from("node-1"));
//         let _ = dkg_engine_node1.ack_partial_commitment(String::from("node-2"));
//         let _ = dkg_engine_node1.ack_partial_commitment(String::from("node-3"));

//         let result = dkg_engine_node1.handle_ack_messages();

//         assert!(result.is_ok());
//         assert!(is_enum_variant!(result, Ok(())));
//     }

//     #[tokio::test]
//     async fn successful_generations_of_key_sets() {
//         let mut dkg_engines = generate_dkg_engines(5, NodeType::MasterNode).await;
//         let mut dkg_engine_node4 = dkg_engines.pop().unwrap();
//         let mut dkg_engine_node3 = dkg_engines.pop().unwrap();
//         let mut dkg_engine_node2 = dkg_engines.pop().unwrap();
//         let mut dkg_engine_node1 = dkg_engines.pop().unwrap();

//         let (part_commitment_node1, id_1) =
//             dkg_engine_node1.generate_partial_commitment(1).unwrap();

//         let (part_commitment_node2, id_2) =
//             dkg_engine_node2.generate_partial_commitment(1).unwrap();

//         let (part_commitment_node3, id_3) =
//             dkg_engine_node3.generate_partial_commitment(1).unwrap();

//         let (part_commitment_node4, id_4) =
//             dkg_engine_node4.generate_partial_commitment(1).unwrap();

//         let part_commitment_tuples = vec![
//             (part_commitment_node1, id_1),
//             (part_commitment_node2, id_2),
//             (part_commitment_node3, id_3),
//             (part_commitment_node4, id_4),
//         ];

//         for (part, node_id) in part_commitment_tuples.iter() {
//             if node_id.to_string() != dkg_engine_node1.node_id() {
//                 dkg_engine_node1
//                     .dkg_state
//                     .part_message_store_mut()
//                     .insert(node_id.to_owned(), part.clone());
//             }

//             if node_id.to_string() != dkg_engine_node2.node_id() {
//                 dkg_engine_node2
//                     .dkg_state
//                     .part_message_store_mut()
//                     .insert(node_id.to_owned(), part.clone());
//             }

//             if node_id.to_string() != dkg_engine_node3.node_id() {
//                 dkg_engine_node3
//                     .dkg_state
//                     .part_message_store_mut()
//                     .insert(node_id.to_owned(), part.clone());
//             }

//             if node_id.to_string() != dkg_engine_node4.node_id() {
//                 dkg_engine_node4
//                     .dkg_state
//                     .part_message_store_mut()
//                     .insert(node_id.to_owned(), part.clone());
//             }
//         }

//         // println!(
//         //     "Node 1{:?}",
//         //     dkg_engine_node1.dkg_state.part_message_store()
//         // );
//         //
//         // println!(
//         //     "Node 2{:?}",
//         //     dkg_engine_node2.dkg_state.part_message_store()
//         // );
//         //
//         // println!(
//         //     "Node 3{:?}",
//         //     dkg_engine_node3.dkg_state.part_message_store()
//         // );
//         //
//         // println!(
//         //     "Node 4{:?}",
//         //     dkg_engine_node4.dkg_state.part_message_store()
//         // );
//         //
//         // let dkg_engine_node1_acks=vec![];
//         for i in 1..=4 {
//             let _ = dkg_engine_node1
//                 .ack_partial_commitment(format!("node-{}", i))
//                 .unwrap();
//             //println!("Node 1 Ack Part Committment status {:?}", s);

//             let _ = dkg_engine_node2
//                 .ack_partial_commitment(format!("node-{}", i))
//                 .unwrap();
//             //  println!("Node 2 Ack Part Committment status {:?}", s);

//             let _ = dkg_engine_node3
//                 .ack_partial_commitment(format!("node-{}", i))
//                 .unwrap();
//             //println!("Node 3 Ack Part Committment status{:?}", s);

//             let _ = dkg_engine_node4
//                 .ack_partial_commitment(format!("node-{}", i))
//                 .unwrap();
//             // println!("Node 4 Ack Part Committment status {:?}", s);
//         }

//         // println!(
//         //     "Node 1 {:?}",
//         //     dkg_engine_node1.dkg_state.ack_message_store()
//         // );
//         //
//         // println!(
//         //     "Node 2 {:?}",
//         //     dkg_engine_node2.dkg_state.ack_message_store()
//         // );
//         //
//         // println!(
//         //     "Node 3 {:?}",
//         //     dkg_engine_node3.dkg_state.ack_message_store()
//         // );
//         //
//         // println!(
//         //     "Node 4 {:?}",
//         //     dkg_engine_node4.dkg_state.ack_message_store()
//         // );

//         let mut new_store: HashMap<(NodeId, SenderId), Ack>;

//         new_store = dkg_engine_node1
//             .dkg_state
//             .ack_message_store_mut()
//             .clone()
//             .into_iter()
//             .chain(dkg_engine_node2.dkg_state.ack_message_store().clone())
//             .collect();

//         new_store = new_store
//             .into_iter()
//             .chain(dkg_engine_node3.dkg_state.ack_message_store().clone())
//             .collect();

//         new_store = new_store
//             .into_iter()
//             .chain(dkg_engine_node4.dkg_state.ack_message_store().clone())
//             .collect();

//         dkg_engine_node1
//             .dkg_state
//             .set_ack_message_store(new_store.clone());

//         dkg_engine_node2
//             .dkg_state
//             .set_ack_message_store(new_store.clone());

//         dkg_engine_node3
//             .dkg_state
//             .set_ack_message_store(new_store.clone());

//         dkg_engine_node4
//             .dkg_state
//             .set_ack_message_store(new_store.clone());

//         for _ in 0..4 {
//             dkg_engine_node1.handle_ack_messages().unwrap();
//             dkg_engine_node2.handle_ack_messages().unwrap();
//             dkg_engine_node3.handle_ack_messages().unwrap();
//             dkg_engine_node4.handle_ack_messages().unwrap();
//         }

//         let result = dkg_engine_node1.generate_key_sets();

//         assert!(result.is_ok());
//         assert!(dkg_engine_node1.dkg_state.public_key_set().is_some());
//         assert!(dkg_engine_node1.dkg_state.secret_key_share().is_some());
//     }

//     fn add_part_commitment_to_node_dkg_state(
//         dkg_engine_node1: &mut DkgEngine,
//         dkg_engine_node2: &mut DkgEngine,
//         node_id: NodeId,
//     ) {
//         let (part, _) = dkg_engine_node2.generate_partial_commitment(1).unwrap();

//         dkg_engine_node1
//             .dkg_state
//             .part_message_store_mut()
//             .insert(node_id, part);
//     }
// }
