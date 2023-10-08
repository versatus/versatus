//! Test that Harvesters, upon receiving the `VALIDATION_THRESHOLD` of `Certificates` from
//! fellow members of the `HarvesterQuorum` form a proper `Certificate`.
//!
//! These tests only certify that this exchange is happening locally.
//!
//! Integration tests are needed for testing that these `Certificate`s are broadcasted.

use block::{header::BlockHeader, Certificate, ConvergenceBlock};
use events::DEFAULT_BUFFER;
use node::{
    node_runtime::NodeRuntime,
    test_utils::{create_quorum_assigned_node_runtime_network, produce_random_claim},
    NodeError,
};
use primitives::{QuorumKind, Signature};

#[tokio::test]
#[serial_test::serial]
async fn harvester_nodes_form_certificate() {
    let (events_tx, _rx) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);
    let nodes = create_quorum_assigned_node_runtime_network(8, 3, events_tx.clone()).await;

    let mut harvesters: Vec<NodeRuntime> = nodes
        .into_iter()
        .filter_map(|nr| {
            if nr.consensus_driver.quorum_kind() == Some(QuorumKind::Harvester) {
                Some(nr)
            } else {
                None
            }
        })
        .collect();
    let mut convergence_block = dummy_convergence_block();
    let mut chosen_harvester = harvesters.pop().unwrap();
    chosen_harvester
        .state_driver
        .append_convergence(&mut convergence_block)
        .map_err(|err| {
            NodeError::Other(format!(
                "Could not append convergence block to DAG: {err:?}"
            ))
        })
        .unwrap();
    let mut sigs: Vec<Signature> = Vec::new();
    for harvester in harvesters.iter_mut() {
        // 2 of 3 harvester nodes sign a convergence block
        sigs.push(
            harvester
                .handle_sign_convergence_block(convergence_block.clone())
                .await
                .unwrap(),
        );
        harvester
            .state_driver
            .append_convergence(&mut convergence_block.clone())
            .map_err(|err| {
                NodeError::Other(format!(
                    "Could not append convergence block to DAG: {err:?}"
                ))
            })
            .unwrap();
    }
    let mut res: Result<Certificate, NodeError> = Err(NodeError::Other("".to_string()));
    // all harvester nodes get the other's signatures
    for (sig, harvester) in sigs.into_iter().zip(harvesters.iter()) {
        res = chosen_harvester
            .handle_harvester_signature_received(
                convergence_block.hash.clone(),
                harvester.config.id.clone(),
                sig,
            )
            .await;
    }

    // ensure they form a full certificate
    assert!(res.is_ok());
}

fn dummy_convergence_block() -> ConvergenceBlock {
    ConvergenceBlock {
        header: BlockHeader {
            ref_hashes: Default::default(),
            epoch: Default::default(),
            round: Default::default(),
            block_seed: Default::default(),
            next_block_seed: Default::default(),
            block_height: Default::default(),
            timestamp: Default::default(),
            txn_hash: Default::default(),
            miner_claim: produce_random_claim(),
            claim_list_hash: Default::default(),
            block_reward: Default::default(),
            next_block_reward: Default::default(),
            miner_signature: Default::default(),
        },
        txns: Default::default(),
        claims: Default::default(),
        hash: "dummy_convergence_block".into(),
        certificate: None,
    }
}
