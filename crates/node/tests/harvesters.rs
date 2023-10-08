//! Test that Harvesters, upon receiving the `VALIDATION_THRESHOLD` of `Certificates` from
//! fellow members of the `HarvesterQuorum` form a proper `Certificate`.
//!
//! These tests only certify that this exchange is happening locally.
//!
//! Integration tests are needed for testing that these `Certificate`s are broadcasted.

use node::{node_runtime::NodeRuntime, test_utils::create_quorum_assigned_node_runtime_network};
use primitives::QuorumKind;

#[tokio::test]
async fn harvester_nodes_form_certificate() {
    let nodes = create_quorum_assigned_node_runtime_network(8, 3).await;

    let mut harvesters: Vec<&NodeRuntime> = nodes
        .iter()
        .filter_map(|nr| {
            if nr.consensus_driver.quorum_kind() == Some(QuorumKind::Harvester) {
                Some(nr)
            } else {
                None
            }
        })
        .collect();
    for harvester in harvesters.iter_mut().skip(1) {
        // create sig A & B
    }

    // A B & C call the method on those sigs
    // ensure they form a full certificate
}
