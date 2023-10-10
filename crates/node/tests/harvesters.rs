//! Test that Harvesters, upon receiving the `VALIDATION_THRESHOLD` of `Certificates` from
//! fellow members of the `HarvesterQuorum` form a proper `Certificate`.
//!
//! These tests only certify that this exchange is happening locally.
//!
//! Integration tests are needed for testing that these `Certificate`s are broadcasted.

use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use block::{header::BlockHeader, Block, Certificate, ConvergenceBlock, ProposalBlock};
use bulldag::vertex::Vertex;
use events::DEFAULT_BUFFER;
use node::{
    node_runtime::NodeRuntime,
    test_utils::{
        create_quorum_assigned_node_runtime_network, produce_random_claim, produce_random_claims, produce_proposal_blocks,
    },
    NodeError,
};
use primitives::{QuorumKind, Signature};
use quorum::{election::Election, quorum::Quorum};
use sha256::digest;
use vrrb_core::{
    claim::{Claim, Eligibility},
    keypair::KeyPair,
};

#[tokio::test]
async fn harvesters_can_build_proposal_blocks() {
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
    if let Some(harvester) = harvesters.iter_mut().last() {
        assert!(harvester
            .handle_build_proposal_block_requested(dummy_convergence_block())
            .await
            .is_ok());
    }
}

#[tokio::test]
async fn non_harvesters_cannot_build_proposal_blocks() {
    let (events_tx, _rx) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);
    let nodes = create_quorum_assigned_node_runtime_network(8, 3, events_tx.clone()).await;

    let mut non_harvesters: Vec<NodeRuntime> = nodes
        .into_iter()
        .filter_map(|nr| {
            if nr.consensus_driver.quorum_kind() != Some(QuorumKind::Harvester)
                && !nr.consensus_driver.is_bootstrap_node()
            {
                Some(nr)
            } else {
                None
            }
        })
        .collect();
    let convergence_block = dummy_convergence_block();
    for node in non_harvesters.iter_mut() {
        assert!(node
            .handle_build_proposal_block_requested(convergence_block.clone())
            .await
            .is_err());
    }
}

#[tokio::test]
/// This test proves the functionality of `handle_harvester_signature_received`.
///
/// 2 of 3 harvester nodes sign a convergence block, which all 3 harvesters have
/// appended to state, afterwhich the harvester VALIDATION_THRESHOLD is reached
/// confirmed by the SignerEngine, and forms a complete certificate.
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
    let _ = chosen_harvester
        .state_driver
        .append_convergence(&mut convergence_block);
    let mut sigs: Vec<Signature> = Vec::new();
    for harvester in harvesters.iter_mut() {
        // 2 of 3 harvester nodes sign a convergence block
        sigs.push(
            harvester
                .handle_sign_convergence_block(convergence_block.clone())
                .await
                .unwrap(),
        );
        let _ = harvester
            .state_driver
            .append_convergence(&mut convergence_block.clone());
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

#[tokio::test]
/// Asserts that a full certificate created by harvester nodes contains
/// the pending quorum that formed directly prior to the certificate's creation.
async fn certificate_formed_includes_pending_quorum() {
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

    let _ = chosen_harvester
        .state_driver
        .append_convergence(&mut convergence_block);

    let mut sigs: Vec<Signature> = Vec::new();

    for harvester in harvesters.iter_mut() {
        // 2 of 3 harvester nodes sign a convergence block
        sigs.push(
            harvester
                .handle_sign_convergence_block(convergence_block.clone())
                .await
                .unwrap(),
        );
        let _ = harvester
            .state_driver
            .append_convergence(&mut convergence_block.clone());
    }

    let mut eligible_claims = produce_random_claims(21)
        .into_iter()
        .collect::<Vec<Claim>>();

    eligible_claims
        .iter_mut()
        .for_each(|claim| claim.eligibility = Eligibility::Validator);

    chosen_harvester
        .state_driver
        .insert_claims(eligible_claims)
        .unwrap();

    assert!(chosen_harvester.consensus_driver.is_harvester().is_ok());

    chosen_harvester
        .handle_quorum_election_started(convergence_block.header)
        .unwrap();

    assert!(chosen_harvester.consensus_driver.is_harvester().is_ok());

    let mut res: Result<Certificate, NodeError> = Err(NodeError::Other("".to_string()));
    // all harvester nodes get the other's signatures
    for (sig, harvester) in sigs.into_iter().zip(harvesters.iter()) {
        assert!(harvester.consensus_driver.is_harvester().is_ok());
        res = chosen_harvester
            .handle_harvester_signature_received(
                convergence_block.hash.clone(),
                harvester.config.id.clone(),
                sig,
            )
            .await;
    }

    let cert = res.unwrap();
    assert!(cert.inauguration.is_some());
}

#[tokio::test]
async fn all_nodes_append_certificate_to_convergence_block() {
    let (events_tx, _rx) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);
    let nodes = create_quorum_assigned_node_runtime_network(8, 3, events_tx.clone()).await;

    let mut harvesters: Vec<NodeRuntime> = nodes
        .iter()
        .filter_map(|nr| {
            if nr.consensus_driver.quorum_kind() == Some(QuorumKind::Harvester) {
                Some(nr.clone())
            } else {
                None
            }
        })
        .collect();
    let mut all_nodes: Vec<NodeRuntime> = nodes
        .into_iter()
        .filter_map(|nr| {
            if nr.consensus_driver.quorum_kind() != Some(QuorumKind::Harvester)
                && !nr.consensus_driver.is_bootstrap_node()
            {
                Some(nr.clone())
            } else {
                None
            }
        })
        .collect();
    let mut convergence_block = dummy_convergence_block();
    harvesters.iter_mut().for_each(|node| {
        node.state_driver
            .handle_block_received(
                &mut block::Block::Convergence {
                    block: convergence_block.clone(),
                },
                node.consensus_driver.sig_engine(),
            )
            .unwrap();
    });
    all_nodes.iter_mut().for_each(|node| {
        node.state_driver
            .handle_block_received(
                &mut block::Block::Convergence {
                    block: convergence_block.clone(),
                },
                node.consensus_driver.sig_engine(),
            )
            .unwrap();
    });
    let mut chosen_harvester = harvesters.pop().unwrap();
    let _ = chosen_harvester
        .state_driver
        .append_convergence(&mut convergence_block);
    let mut sigs: Vec<Signature> = Vec::new();
    for harvester in harvesters.iter_mut() {
        // 2 of 3 harvester nodes sign a convergence block
        sigs.push(
            harvester
                .handle_sign_convergence_block(convergence_block.clone())
                .await
                .unwrap(),
        );
        let _ = harvester
            .state_driver
            .append_convergence(&mut convergence_block.clone());
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
    let certificate = res.unwrap();
    all_nodes.extend(harvesters);
    for node in all_nodes.iter_mut() {
        let convergence_block = node
            .handle_block_certificate_received(certificate.clone())
            .await
            .unwrap();
        assert_eq!(&convergence_block.certificate.unwrap(), &certificate);
    }
    let convergence_block = chosen_harvester
        .handle_block_certificate_created(certificate.clone())
        .await
        .unwrap();
    assert_eq!(&convergence_block.certificate.unwrap(), &certificate);
}

#[tokio::test]
async fn all_nodes_append_certified_convergence_block_to_dag() {
    let (events_tx, _rx) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);
    let nodes = create_quorum_assigned_node_runtime_network(8, 3, events_tx.clone()).await;

    let mut harvesters: Vec<NodeRuntime> = nodes
        .iter()
        .filter_map(|nr| {
            if nr.consensus_driver.quorum_kind() == Some(QuorumKind::Harvester) {
                Some(nr.clone())
            } else {
                None
            }
        })
        .collect();
    let mut all_nodes: Vec<NodeRuntime> = nodes
        .into_iter()
        .filter_map(|nr| {
            if nr.consensus_driver.quorum_kind() != Some(QuorumKind::Harvester)
                && !nr.consensus_driver.is_bootstrap_node()
            {
                Some(nr.clone())
            } else {
                None
            }
        })
        .collect();
    let sig_engine = all_nodes[0].consensus_driver.sig_engine();
    let mut proposal_block = dummy_proposal_block(sig_engine);
    let mut convergence_block = dummy_convergence_block();
    convergence_block.header.ref_hashes = vec![proposal_block.hash.clone()];
    let pblock: Block = proposal_block.into();
    let vtx = pblock.into();
    all_nodes.iter_mut().for_each(|node| {
        node.state_driver.write_vertex(&vtx).unwrap();
    });

    harvesters.iter_mut().for_each(|node| {
        node.state_driver.write_vertex(&vtx).unwrap();
        node.state_driver
            .handle_block_received(
                &mut block::Block::Convergence {
                    block: convergence_block.clone(),
                },
                node.consensus_driver.sig_engine(),
            )
            .unwrap();
    });
    all_nodes.iter_mut().for_each(|node| {
        node.state_driver
            .handle_block_received(
                &mut block::Block::Convergence {
                    block: convergence_block.clone(),
                },
                node.consensus_driver.sig_engine(),
            )
            .unwrap();
    });
    let mut chosen_harvester = harvesters.pop().unwrap();
    let _ = chosen_harvester
        .state_driver
        .append_convergence(&mut convergence_block);
    let mut sigs: Vec<Signature> = Vec::new();
    for harvester in harvesters.iter_mut() {
        // 2 of 3 harvester nodes sign a convergence block
        sigs.push(
            harvester
                .handle_sign_convergence_block(convergence_block.clone())
                .await
                .unwrap(),
        );
        let _ = harvester
            .state_driver
            .append_convergence(&mut convergence_block.clone());
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
    let certificate = res.unwrap();
    all_nodes.extend(harvesters);
    for node in all_nodes.iter_mut() {
        let convergence_block = node
            .handle_block_certificate_received(certificate.clone())
            .await
            .unwrap();
        assert_eq!(&convergence_block.certificate.unwrap(), &certificate);
        assert!(node.certified_convergence_block_exists_within_dag(convergence_block.hash));
    }
    let convergence_block = chosen_harvester
        .handle_block_certificate_created(certificate.clone())
        .await
        .unwrap();
    assert_eq!(&convergence_block.certificate.unwrap(), &certificate);
    assert!(chosen_harvester.certified_convergence_block_exists_within_dag(convergence_block.hash));
}

fn dummy_convergence_block() -> ConvergenceBlock {
    let keypair = KeyPair::random();
    let public_key = keypair.get_miner_public_key();
    let mut hasher = DefaultHasher::new();
    public_key.hash(&mut hasher);
    let pubkey_hash = hasher.finish();

    let mut pub_key_bytes = pubkey_hash.to_string().as_bytes().to_vec();
    pub_key_bytes.push(1u8);

    let hash = digest(digest(&*pub_key_bytes).as_bytes());

    let payload = (21_600, hash);
    ConvergenceBlock {
        header: BlockHeader {
            ref_hashes: Default::default(),
            epoch: Default::default(),
            round: Default::default(),
            block_seed: Default::default(),
            next_block_seed: Quorum::generate_seed(payload, keypair).unwrap(),
            block_height: 21_600,
            timestamp: Default::default(),
            txn_hash: Default::default(),
            miner_claim: produce_random_claim(22),
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

fn dummy_proposal_block(sig_engine: signer::engine::SignerEngine) -> ProposalBlock {
    produce_proposal_blocks(
        "dummy_proposal_block".to_string(), vec![], 1, 0, sig_engine
    ).pop().unwrap()
}
