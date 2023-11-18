pub mod component;
pub mod handler_helpers;
pub mod node_runtime;
pub mod node_runtime_handler;
mod setup;

pub use handler_helpers::*;
pub use setup::*;

#[cfg(test)]
mod tests {

    use crate::node_runtime::NodeRuntime;
    use crate::test_utils::{
        create_node_runtime_network, create_quorum_assigned_node_runtime_network,
        create_sender_receiver_addresses, create_txn_from_accounts,
        create_txn_from_accounts_invalid_signature, create_txn_from_accounts_invalid_timestamp,
        setup_network, setup_whitelisted_nodes,
    };
    use crate::NodeError;
    use block::{Block, GenesisReceiver};
    use events::{AssignedQuorumMembership, PeerData, Vote, DEFAULT_BUFFER};
    use primitives::{generate_account_keypair, Address, NodeId, NodeType, QuorumKind};
    use storage::storage_utils::remove_vrrb_data_dir;
    use vrrb_core::account::{Account, AccountField};
    use vrrb_core::transactions::Transaction;

    #[tokio::test]
    #[serial_test::serial]
    async fn bootstrap_node_runtime_cannot_be_assigned_to_quorum() {
        remove_vrrb_data_dir();
        let (events_tx, _) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);

        let mut nodes = create_node_runtime_network(1, events_tx.clone()).await;
        let mut node = nodes.pop_front().unwrap();
        assert_eq!(node.config.node_type, NodeType::Bootstrap);

        let assigned_membership = AssignedQuorumMembership {
            quorum_kind: QuorumKind::Farmer,
            node_id: node.id.clone(),
            pub_key: node.config.keypair.validator_public_key_owned(),
            kademlia_peer_id: node.config.kademlia_peer_id.unwrap(),
            peers: vec![],
        };

        let assignment_result =
            node.handle_quorum_membership_assigment_created(assigned_membership);

        assert!(assignment_result.is_err());
        assert!(node.quorum_membership().is_none());
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn validator_node_runtime_can_be_assigned_to_quorum() {
        remove_vrrb_data_dir();
        let (events_tx, _) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);

        let mut nodes = create_node_runtime_network(2, events_tx.clone()).await;
        nodes.pop_front().unwrap();
        let mut node = nodes.pop_front().unwrap();
        assert_eq!(node.config.node_type, NodeType::Validator);

        let assigned_membership = AssignedQuorumMembership {
            quorum_kind: QuorumKind::Farmer,
            node_id: node.id.clone(),
            pub_key: node.config.keypair.validator_public_key_owned(),
            kademlia_peer_id: node.config.kademlia_peer_id.unwrap(),
            peers: vec![],
        };

        let assignment_result =
            node.handle_quorum_membership_assigment_created(assigned_membership);

        assert!(assignment_result.is_ok());
        assert!(node.quorum_membership().is_some());
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn bootstrap_node_runtime_can_assign_quorum_memberships_to_available_nodes() {
        let (node_0, farmers, harvesters, miners) = setup_network(8).await;

        assert_eq!(farmers.len(), 4);
        assert_eq!(harvesters.len(), 2);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn bootstrap_node_runtime_can_produce_genesis_reward() {
        let (node_0, farmers, harvesters, miners) = setup_network(8).await;
        assert!(node_0.distribute_genesis_reward(vec![]).is_err());

        for (_, node) in farmers.iter() {
            assert!(node.distribute_genesis_reward(vec![]).is_err());
        }

        for (_, node) in harvesters.iter() {
            assert!(node.distribute_genesis_reward(vec![]).is_err());
        }

        for (_, node) in miners.iter() {
            assert!(node.distribute_genesis_reward(vec![]).is_ok());
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn assigned_quorum_members_exist_in_sig_engine() {
        let (_node_0, farmers, harvesters, _miners) = setup_network(8).await;
        let mut validators = farmers.clone();
        validators.extend(harvesters.clone().into_iter());

        for (farmer_id, farmer) in farmers.iter() {
            for (validator_id, member) in validators.iter() {
                if validator_id == farmer_id {
                    continue;
                }
                assert!(farmer
                    .consensus_driver
                    .sig_engine
                    .quorum_members()
                    .get_public_key_from_members(&member.config.id)
                    .is_some());
            }
        }
        for (harvester_id, harvester) in harvesters.iter() {
            for (validator_id, member) in validators.iter() {
                if validator_id == harvester_id {
                    continue;
                }
                assert!(harvester
                    .consensus_driver
                    .sig_engine
                    .quorum_members()
                    .get_public_key_from_members(&member.config.id)
                    .is_some());
            }
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn miner_node_runtime_can_mine_genesis_block() {
        let (node_0, farmers, harvesters, mut miners) = setup_network(8).await;

        let whitelisted_nodes = setup_whitelisted_nodes(&farmers, &harvesters, &miners);

        let miner_ids = miners
            .clone()
            .into_iter()
            .map(|(key, _)| key)
            .collect::<Vec<NodeId>>();

        let miner_id = miner_ids.first().unwrap();

        let miner_node = miners.get(miner_id).unwrap();
        let receiver = GenesisReceiver(Address::new(
            farmers
                .iter()
                .last()
                .unwrap()
                .1
                .config
                .keypair
                .miner_public_key_owned(),
        ));
        let genesis_rewards = miner_node
            .distribute_genesis_reward(vec![receiver])
            .unwrap();
        let miner_node = miners.get_mut(miner_id).unwrap();

        let harvester_ids = harvesters.keys().cloned().collect::<Vec<NodeId>>();
        let harvester_id = harvester_ids.first().unwrap();
        let mut harvester = harvesters.get(harvester_id).unwrap().clone();

        miner_node.config_mut().whitelisted_nodes = whitelisted_nodes;

        assert!(node_0.mine_genesis_block(genesis_rewards.clone()).is_err());

        for harvester in harvesters.values() {
            assert!(harvester
                .mine_genesis_block(genesis_rewards.clone())
                .is_err());
        }

        for farmer in farmers.values() {
            assert!(farmer.mine_genesis_block(genesis_rewards.clone()).is_err());
        }

        let block = miner_node.mine_genesis_block(genesis_rewards).unwrap();

        harvester
            .handle_block_received(block::Block::from(block))
            .unwrap();
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn farmer_node_runtime_can_validate_transactions() {
        let (mut node_0, mut farmers, _harvesters, _miners) = setup_network(8).await;

        let (_, sender_public_key) = generate_account_keypair();
        let sender_account = Account::new(sender_public_key.clone().into());
        let sender_address = node_0.create_account(sender_public_key).unwrap();

        let (_, receiver_public_key) = generate_account_keypair();
        let receiver_address = node_0.create_account(receiver_public_key).unwrap();

        let txn = create_txn_from_accounts(
            (sender_address, Some(sender_account)),
            receiver_address,
            vec![],
        );

        for (_node_id, farmer) in farmers.iter_mut() {
            let _ = farmer.insert_txn_to_mempool(txn.clone());
            farmer
                .validate_transaction_kind(
                    txn.id(),
                    farmer.mempool_read_handle_factory().clone(),
                    farmer.state_store_read_handle_factory().clone(),
                )
                .unwrap();
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn harvester_node_runtime_can_propose_blocks() {
        let (mut node_0, farmers, mut harvesters, mut miners) = setup_network(8).await;
        node_0.config.node_type = NodeType::Miner;
        let receiver = GenesisReceiver(Address::new(
            farmers
                .iter()
                .last()
                .unwrap()
                .1
                .config
                .keypair
                .miner_public_key_owned(),
        ));
        let genesis_rewards = node_0.distribute_genesis_reward(vec![receiver]).unwrap();

        let whitelisted_nodes = setup_whitelisted_nodes(&farmers, &harvesters, &miners);

        for (_, harvester) in harvesters.iter_mut() {
            harvester.config_mut().whitelisted_nodes = whitelisted_nodes.clone();
        }

        for (_, miner_node) in miners.iter_mut() {
            miner_node.config_mut().whitelisted_nodes = whitelisted_nodes.clone();
        }

        let miner_ids = miners
            .clone()
            .into_iter()
            .map(|(key, _)| key)
            .collect::<Vec<NodeId>>();

        let miner_id = miner_ids.first().unwrap();

        let mut miner_node = miners.get(miner_id).unwrap().to_owned();

        let claim = miner_node.state_driver.dag.claim();

        let genesis_block = miner_node.mine_genesis_block(genesis_rewards).unwrap();

        // TODO: impl miner elections
        // TODO: create genesis block, certify it then append it to miner's dag
        // TODO: store DAG on disk, separate from ledger

        let (_, public_key) = generate_account_keypair();
        let sender_account = Account::new(public_key.clone().into());
        let sender_address = node_0.create_account(public_key).unwrap();

        let (_, public_key) = generate_account_keypair();
        let receiver_address = node_0.create_account(public_key).unwrap();

        let txn = create_txn_from_accounts(
            (sender_address, Some(sender_account)),
            receiver_address,
            vec![],
        );

        let mut apply_results = Vec::new();

        for (_, harvester) in harvesters.iter_mut() {
            let apply_result = harvester
                .handle_block_received(Block::Genesis {
                    block: genesis_block.clone(),
                })
                .unwrap();

            // let genesis_cert = harvester
            //     .certify_genesis_block(genesis_block.clone())
            //     .unwrap();

            apply_results.push(apply_result);
            // genesis_certs.push(genesis_cert);
        }

        miner_node
            .handle_block_received(Block::Genesis {
                block: genesis_block.clone(),
            })
            .unwrap();

        for (_, harvester) in harvesters.iter_mut() {
            let sig_engine = harvester.consensus_driver.sig_engine.clone();
            let proposal_block = harvester
                .mine_proposal_block(
                    genesis_block.hash.clone(),
                    Default::default(), // TODO: change to an actual map of harvester claims
                    1,
                    1,
                    claim.clone(),
                    sig_engine.clone(),
                )
                .unwrap();
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn harvester_node_runtime_can_handle_genesis_block_created() {
        let (mut node_0, farmers, mut harvesters, miners) = setup_network(8).await;
        node_0.config.node_type = NodeType::Miner;
        let receiver = GenesisReceiver(Address::new(
            farmers
                .iter()
                .last()
                .unwrap()
                .1
                .config
                .keypair
                .miner_public_key_owned(),
        ));
        let genesis_rewards = node_0.distribute_genesis_reward(vec![receiver]).unwrap();

        let miner_ids = miners
            .clone()
            .into_iter()
            .map(|(key, _)| key)
            .collect::<Vec<NodeId>>();

        let miner_id = miner_ids.first().unwrap();

        let miner_node = miners.get(miner_id).unwrap();

        let genesis_block = miner_node.mine_genesis_block(genesis_rewards).unwrap();

        let mut apply_results = Vec::new();

        for (_, harvester) in harvesters.iter_mut() {
            let apply_result = harvester
                .handle_block_received(Block::Genesis {
                    block: genesis_block.clone(),
                })
                .unwrap();

            apply_results.push(apply_result);
        }

        for (_, harvester) in harvesters.iter_mut() {
            let state_trie_root_hash = harvester.state_root_hash().unwrap();
            for res in apply_results.iter() {
                assert_eq!(state_trie_root_hash, res.state_root_hash_str());
            }
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    #[ignore = "https://github.com/versatus/versatus/issues/488"]
    async fn harvester_node_runtime_can_handle_convergence_block_created() {
        let (mut node_0, farmers, mut harvesters, mut miners) = setup_network(8).await;
        let receiver = GenesisReceiver(Address::new(
            farmers
                .iter()
                .last()
                .unwrap()
                .1
                .config
                .keypair
                .miner_public_key_owned(),
        ));
        let genesis_rewards = node_0.distribute_genesis_reward(vec![receiver]).unwrap();

        let miner_ids = miners
            .clone()
            .into_iter()
            .map(|(key, _)| key)
            .collect::<Vec<NodeId>>();

        let miner_id = miner_ids.first().unwrap();

        let mut miner_node = miners.get_mut(miner_id).unwrap();

        let genesis_block = miner_node.mine_genesis_block(genesis_rewards).unwrap();

        // TODO: impl miner elections
        // TODO: create genesis block, certify it then append it to miner's dag
        // TODO: store DAG on disk, separate from ledger

        let mut apply_results = Vec::new();
        // let mut genesis_certs = Vec::new();

        for (_, harvester) in harvesters.iter_mut() {
            let apply_result = harvester
                .handle_block_received(Block::Genesis {
                    block: genesis_block.clone(),
                })
                .unwrap();

            // let genesis_cert = harvester
            //     .certify_genesis_block(genesis_block.clone())
            //     .unwrap();

            apply_results.push(apply_result);
            // genesis_certs.push(genesis_cert);
        }

        miner_node
            .handle_block_received(Block::Genesis {
                block: genesis_block.clone(),
            })
            .unwrap();

        let convergence_block = miner_node.mine_convergence_block().unwrap();

        let mut apply_results = Vec::new();

        for (_, harvester) in harvesters.iter_mut() {
            let apply_result = harvester
                .handle_block_received(Block::Convergence {
                    block: convergence_block.clone(),
                })
                .unwrap();

            apply_results.push(apply_result);
        }

        for (_, harvester) in harvesters.iter_mut() {
            let txn_trie_root_hash = harvester.transactions_root_hash().unwrap();
            let state_trie_root_hash = harvester.state_root_hash().unwrap();
            for res in apply_results.iter() {
                assert_eq!(txn_trie_root_hash, res.transactions_root_hash_str());
                assert_eq!(state_trie_root_hash, res.state_root_hash_str());
            }
        }
        panic!();
    }

    #[tokio::test]
    #[ignore = "broken atm"]
    async fn node_runtime_can_form_quorum_with_valid_config() {
        let (mut node_0, farmers, harvesters, miners) = setup_network(8).await;

        // let res = node_0.generate_partial_commitment_message();
        // assert!(res.is_err(), "bootstrap nodes cannot participate in DKG");

        //run_dkg_process(farmers);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn farmer_node_runtime_can_form_valid_vote_on_valid_transaction() {
        let (events_tx, _rx) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);

        let mut nodes = create_node_runtime_network(4, events_tx.clone()).await;
        // NOTE: remove bootstrap
        nodes.pop_front().unwrap();

        let mut node_1 = nodes.pop_front().unwrap();
        assert_eq!(node_1.config.node_type, NodeType::Validator);

        let mut node_2 = nodes.pop_front().unwrap();
        assert_eq!(node_2.config.node_type, NodeType::Validator);

        let node_1_peer_data = PeerData {
            node_id: node_1.config.id.clone(),
            node_type: node_1.config.node_type,
            kademlia_peer_id: node_1.config.kademlia_peer_id.unwrap(),
            udp_gossip_addr: node_1.config.udp_gossip_address,
            raptorq_gossip_addr: node_1.config.raptorq_gossip_address,
            kademlia_liveness_addr: node_1.config.kademlia_liveness_address,
            validator_public_key: node_1.config.keypair.validator_public_key_owned(),
        };

        let node_2_peer_data = PeerData {
            node_id: node_2.config.id.clone(),
            node_type: node_2.config.node_type,
            kademlia_peer_id: node_2.config.kademlia_peer_id.unwrap(),
            udp_gossip_addr: node_2.config.udp_gossip_address,
            raptorq_gossip_addr: node_2.config.raptorq_gossip_address,
            kademlia_liveness_addr: node_2.config.kademlia_liveness_address,
            validator_public_key: node_2.config.keypair.validator_public_key_owned(),
        };
        node_1
            .handle_node_added_to_peer_list(node_2_peer_data.clone())
            .await
            .unwrap();
        assert!(node_1
            .consensus_driver
            .quorum_driver
            .bootstrap_quorum_available_nodes
            .contains_key(&node_2_peer_data.node_id));

        node_2
            .handle_node_added_to_peer_list(node_1_peer_data.clone())
            .await
            .unwrap();
        assert!(node_2
            .consensus_driver
            .quorum_driver
            .bootstrap_quorum_available_nodes
            .contains_key(&node_1_peer_data.node_id));

        let assigned_membership_1 = AssignedQuorumMembership {
            quorum_kind: QuorumKind::Farmer,
            node_id: node_1.id.clone(),
            pub_key: node_1.config.keypair.validator_public_key_owned(),
            kademlia_peer_id: node_1.config.kademlia_peer_id.unwrap(),
            peers: vec![node_2_peer_data],
        };

        let assigned_membership_2 = AssignedQuorumMembership {
            quorum_kind: QuorumKind::Farmer,
            node_id: node_2.id.clone(),
            pub_key: node_2.config.keypair.validator_public_key_owned(),
            kademlia_peer_id: node_2.config.kademlia_peer_id.unwrap(),
            peers: vec![node_1_peer_data],
        };

        let assignments = vec![assigned_membership_1.clone(), assigned_membership_2.clone()];

        node_1
            .handle_quorum_membership_assigment_created(assigned_membership_1)
            .unwrap();

        node_2
            .handle_quorum_membership_assigment_created(assigned_membership_2)
            .unwrap();

        node_1
            .handle_quorum_membership_assigments_created(assignments.clone())
            .unwrap();

        node_2
            .handle_quorum_membership_assigments_created(assignments.clone())
            .unwrap();

        assert!(node_1
            .consensus_driver
            .quorum_driver
            .bootstrap_quorum_config
            .is_some());

        assert!(node_1
            .consensus_driver
            .sig_engine
            .quorum_members()
            .get_public_key_from_members(&node_1.config.id)
            .is_some());

        let mut farmer_nodes = vec![&mut node_1, &mut node_2];

        let mut node_0 = nodes.pop_front().unwrap();

        let (_, sender_public_key) = generate_account_keypair();
        let mut sender_account = Account::new(sender_public_key.clone().into());
        let update_field = AccountField::Credits(100000);
        let _ = sender_account.update_field(update_field);
        let sender_address = node_0.create_account(sender_public_key).unwrap();

        let (_, receiver_public_key) = generate_account_keypair();
        let receiver_account = Account::new(receiver_public_key.clone().into());
        let receiver_address = node_0.create_account(receiver_public_key).unwrap();

        let sender_account_bytes = bincode::serialize(&sender_account.clone()).unwrap();
        let receiver_account_bytes = bincode::serialize(&receiver_account.clone()).unwrap();

        for farmer in farmer_nodes.iter_mut() {
            let _ = farmer.handle_create_account_requested(
                sender_address.clone(),
                sender_account_bytes.clone(),
            );

            let _ = farmer.handle_create_account_requested(
                receiver_address.clone(),
                receiver_account_bytes.clone(),
            );
        }

        let txn = create_txn_from_accounts(
            (sender_address, Some(sender_account)),
            receiver_address,
            vec![],
        );

        for farmer in farmer_nodes.iter() {
            // dbg!(&farmer.consensus_driver.quorum_driver.node_config.node_type);
            // dbg!(&farmer.consensus_driver.is_farmer());
        }

        for farmer in farmer_nodes.iter_mut() {
            let _ = farmer.insert_txn_to_mempool(txn.clone());
            let (transaction_kind, validity) = farmer
                .validate_transaction_kind(
                    txn.id(),
                    farmer.mempool_read_handle_factory().clone(),
                    farmer.state_store_read_handle_factory().clone(),
                )
                .unwrap();
            assert!(validity);
            farmer
                .cast_vote_on_transaction_kind(transaction_kind, validity)
                .unwrap();
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn farmer_node_runtime_can_form_invalid_vote_on_invalid_transaction_amount_greater_than_balance(
    ) {
        let (events_tx, _rx) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);

        let mut nodes = create_node_runtime_network(4, events_tx.clone()).await;

        // NOTE: remove bootstrap
        nodes.pop_front().unwrap();

        let mut node_1 = nodes.pop_front().unwrap();
        assert_eq!(node_1.config.node_type, NodeType::Validator);

        let mut node_2 = nodes.pop_front().unwrap();
        assert_eq!(node_2.config.node_type, NodeType::Validator);

        let node_1_peer_data = PeerData {
            node_id: node_1.config.id.clone(),
            node_type: node_1.config.node_type,
            kademlia_peer_id: node_1.config.kademlia_peer_id.unwrap(),
            udp_gossip_addr: node_1.config.udp_gossip_address,
            raptorq_gossip_addr: node_1.config.raptorq_gossip_address,
            kademlia_liveness_addr: node_1.config.kademlia_liveness_address,
            validator_public_key: node_1.config.keypair.validator_public_key_owned(),
        };

        let node_2_peer_data = PeerData {
            node_id: node_2.config.id.clone(),
            node_type: node_2.config.node_type,
            kademlia_peer_id: node_2.config.kademlia_peer_id.unwrap(),
            udp_gossip_addr: node_2.config.udp_gossip_address,
            raptorq_gossip_addr: node_2.config.raptorq_gossip_address,
            kademlia_liveness_addr: node_2.config.kademlia_liveness_address,
            validator_public_key: node_2.config.keypair.validator_public_key_owned(),
        };

        node_1
            .handle_node_added_to_peer_list(node_2_peer_data.clone())
            .await
            .unwrap();

        node_2
            .handle_node_added_to_peer_list(node_1_peer_data.clone())
            .await
            .unwrap();

        let assigned_membership_1 = AssignedQuorumMembership {
            quorum_kind: QuorumKind::Farmer,
            node_id: node_1.id.clone(),
            pub_key: node_1.config.keypair.validator_public_key_owned(),
            kademlia_peer_id: node_1.config.kademlia_peer_id.unwrap(),
            peers: vec![node_2_peer_data],
        };

        node_1
            .handle_quorum_membership_assigment_created(assigned_membership_1)
            .unwrap();

        let assigned_membership_2 = AssignedQuorumMembership {
            quorum_kind: QuorumKind::Farmer,
            node_id: node_2.id.clone(),
            pub_key: node_2.config.keypair.validator_public_key_owned(),
            kademlia_peer_id: node_2.config.kademlia_peer_id.unwrap(),
            peers: vec![node_1_peer_data],
        };

        node_2
            .handle_quorum_membership_assigment_created(assigned_membership_2)
            .unwrap();

        let mut farmer_nodes = vec![&mut node_1, &mut node_2];

        let mut node_0 = nodes.pop_front().unwrap();

        let (_, sender_public_key) = generate_account_keypair();
        let mut sender_account = Account::new(sender_public_key.clone().into());
        let update_field = AccountField::Credits(100);
        let _ = sender_account.update_field(update_field);
        let sender_address = node_0.create_account(sender_public_key).unwrap();

        let (_, receiver_public_key) = generate_account_keypair();
        let receiver_account = Account::new(receiver_public_key.clone().into());
        let receiver_address = node_0.create_account(receiver_public_key).unwrap();

        let sender_account_bytes = bincode::serialize(&sender_account.clone()).unwrap();
        let receiver_account_bytes = bincode::serialize(&receiver_account.clone()).unwrap();

        for farmer in farmer_nodes.iter_mut() {
            let _ = farmer.handle_create_account_requested(
                sender_address.clone(),
                sender_account_bytes.clone(),
            );

            let _ = farmer.handle_create_account_requested(
                receiver_address.clone(),
                receiver_account_bytes.clone(),
            );
        }

        let txn = create_txn_from_accounts(
            (sender_address, Some(sender_account)),
            receiver_address,
            vec![],
        );

        for farmer in farmer_nodes.iter_mut() {
            let _ = farmer.insert_txn_to_mempool(txn.clone());
            let (transaction_kind, validity) = farmer
                .validate_transaction_kind(
                    txn.id(),
                    farmer.mempool_read_handle_factory().clone(),
                    farmer.state_store_read_handle_factory().clone(),
                )
                .unwrap();
            assert!(!validity);
            farmer
                .cast_vote_on_transaction_kind(transaction_kind, validity)
                .unwrap();
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn farmer_node_runtime_can_form_invalid_vote_on_invalid_transaction_invalid_signature() {
        let (events_tx, _rx) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);

        let mut nodes = create_node_runtime_network(4, events_tx.clone()).await;

        // NOTE: remove bootstrap
        nodes.pop_front().unwrap();

        let mut node_1 = nodes.pop_front().unwrap();
        assert_eq!(node_1.config.node_type, NodeType::Validator);

        let mut node_2 = nodes.pop_front().unwrap();
        assert_eq!(node_2.config.node_type, NodeType::Validator);

        let node_1_peer_data = PeerData {
            node_id: node_1.config.id.clone(),
            node_type: node_1.config.node_type,
            kademlia_peer_id: node_1.config.kademlia_peer_id.unwrap(),
            udp_gossip_addr: node_1.config.udp_gossip_address,
            raptorq_gossip_addr: node_1.config.raptorq_gossip_address,
            kademlia_liveness_addr: node_1.config.kademlia_liveness_address,
            validator_public_key: node_1.config.keypair.validator_public_key_owned(),
        };

        let node_2_peer_data = PeerData {
            node_id: node_2.config.id.clone(),
            node_type: node_2.config.node_type,
            kademlia_peer_id: node_2.config.kademlia_peer_id.unwrap(),
            udp_gossip_addr: node_2.config.udp_gossip_address,
            raptorq_gossip_addr: node_2.config.raptorq_gossip_address,
            kademlia_liveness_addr: node_2.config.kademlia_liveness_address,
            validator_public_key: node_2.config.keypair.validator_public_key_owned(),
        };

        node_1
            .handle_node_added_to_peer_list(node_2_peer_data.clone())
            .await
            .unwrap();

        node_2
            .handle_node_added_to_peer_list(node_1_peer_data.clone())
            .await
            .unwrap();

        let assigned_membership_1 = AssignedQuorumMembership {
            quorum_kind: QuorumKind::Farmer,
            node_id: node_1.id.clone(),
            pub_key: node_1.config.keypair.validator_public_key_owned(),
            kademlia_peer_id: node_1.config.kademlia_peer_id.unwrap(),
            peers: vec![node_2_peer_data],
        };

        node_1
            .handle_quorum_membership_assigment_created(assigned_membership_1)
            .unwrap();

        let assigned_membership_2 = AssignedQuorumMembership {
            quorum_kind: QuorumKind::Farmer,
            node_id: node_2.id.clone(),
            pub_key: node_2.config.keypair.validator_public_key_owned(),
            kademlia_peer_id: node_2.config.kademlia_peer_id.unwrap(),
            peers: vec![node_1_peer_data],
        };

        node_2
            .handle_quorum_membership_assigment_created(assigned_membership_2)
            .unwrap();

        let mut farmer_nodes = vec![&mut node_1, &mut node_2];

        let mut node_0 = nodes.pop_front().unwrap();

        let (_, sender_public_key) = generate_account_keypair();
        let mut sender_account = Account::new(sender_public_key.clone().into());
        let update_field = AccountField::Credits(100000);
        let _ = sender_account.update_field(update_field);
        let sender_address = node_0.create_account(sender_public_key).unwrap();

        let (_, receiver_public_key) = generate_account_keypair();
        let receiver_account = Account::new(receiver_public_key.clone().into());
        let receiver_address = node_0.create_account(receiver_public_key).unwrap();

        let sender_account_bytes = bincode::serialize(&sender_account.clone()).unwrap();
        let receiver_account_bytes = bincode::serialize(&receiver_account.clone()).unwrap();

        for farmer in farmer_nodes.iter_mut() {
            let _ = farmer.handle_create_account_requested(
                sender_address.clone(),
                sender_account_bytes.clone(),
            );

            let _ = farmer.handle_create_account_requested(
                receiver_address.clone(),
                receiver_account_bytes.clone(),
            );
        }

        let txn = create_txn_from_accounts_invalid_signature(
            (sender_address, Some(sender_account)),
            receiver_address,
            vec![],
        );

        for farmer in farmer_nodes.iter_mut() {
            let _ = farmer.insert_txn_to_mempool(txn.clone());
            let (transaction_kind, validity) = farmer
                .validate_transaction_kind(
                    txn.id(),
                    farmer.mempool_read_handle_factory().clone(),
                    farmer.state_store_read_handle_factory().clone(),
                )
                .unwrap();
            assert!(!validity);
            farmer
                .cast_vote_on_transaction_kind(transaction_kind, validity)
                .unwrap();
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn farmer_node_runtime_can_form_invalid_vote_on_invalid_transaction_invalid_timestamp() {
        let (events_tx, _rx) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);

        let mut nodes = create_node_runtime_network(4, events_tx.clone()).await;

        // NOTE: remove bootstrap
        nodes.pop_front().unwrap();

        let mut node_1 = nodes.pop_front().unwrap();
        assert_eq!(node_1.config.node_type, NodeType::Validator);

        let mut node_2 = nodes.pop_front().unwrap();
        assert_eq!(node_2.config.node_type, NodeType::Validator);

        let node_1_peer_data = PeerData {
            node_id: node_1.config.id.clone(),
            node_type: node_1.config.node_type,
            kademlia_peer_id: node_1.config.kademlia_peer_id.unwrap(),
            udp_gossip_addr: node_1.config.udp_gossip_address,
            raptorq_gossip_addr: node_1.config.raptorq_gossip_address,
            kademlia_liveness_addr: node_1.config.kademlia_liveness_address,
            validator_public_key: node_1.config.keypair.validator_public_key_owned(),
        };

        let node_2_peer_data = PeerData {
            node_id: node_2.config.id.clone(),
            node_type: node_2.config.node_type,
            kademlia_peer_id: node_2.config.kademlia_peer_id.unwrap(),
            udp_gossip_addr: node_2.config.udp_gossip_address,
            raptorq_gossip_addr: node_2.config.raptorq_gossip_address,
            kademlia_liveness_addr: node_2.config.kademlia_liveness_address,
            validator_public_key: node_2.config.keypair.validator_public_key_owned(),
        };

        node_1
            .handle_node_added_to_peer_list(node_2_peer_data.clone())
            .await
            .unwrap();

        node_2
            .handle_node_added_to_peer_list(node_1_peer_data.clone())
            .await
            .unwrap();

        let assigned_membership_1 = AssignedQuorumMembership {
            quorum_kind: QuorumKind::Farmer,
            node_id: node_1.id.clone(),
            pub_key: node_1.config.keypair.validator_public_key_owned(),
            kademlia_peer_id: node_1.config.kademlia_peer_id.unwrap(),
            peers: vec![node_2_peer_data],
        };

        node_1
            .handle_quorum_membership_assigment_created(assigned_membership_1)
            .unwrap();

        let assigned_membership_2 = AssignedQuorumMembership {
            quorum_kind: QuorumKind::Farmer,
            node_id: node_2.id.clone(),
            pub_key: node_2.config.keypair.validator_public_key_owned(),
            kademlia_peer_id: node_2.config.kademlia_peer_id.unwrap(),
            peers: vec![node_1_peer_data],
        };

        node_2
            .handle_quorum_membership_assigment_created(assigned_membership_2)
            .unwrap();

        let mut farmer_nodes = vec![&mut node_1, &mut node_2];

        let mut node_0 = nodes.pop_front().unwrap();

        let (_, sender_public_key) = generate_account_keypair();
        let mut sender_account = Account::new(sender_public_key.clone().into());
        let update_field = AccountField::Credits(100000);
        let _ = sender_account.update_field(update_field);
        let sender_address = node_0.create_account(sender_public_key).unwrap();

        let (_, receiver_public_key) = generate_account_keypair();
        let receiver_account = Account::new(receiver_public_key.clone().into());
        let receiver_address = node_0.create_account(receiver_public_key).unwrap();

        let sender_account_bytes = bincode::serialize(&sender_account.clone()).unwrap();
        let receiver_account_bytes = bincode::serialize(&receiver_account.clone()).unwrap();

        for farmer in farmer_nodes.iter_mut() {
            let _ = farmer.handle_create_account_requested(
                sender_address.clone(),
                sender_account_bytes.clone(),
            );

            let _ = farmer.handle_create_account_requested(
                receiver_address.clone(),
                receiver_account_bytes.clone(),
            );
        }

        let txn = create_txn_from_accounts_invalid_timestamp(
            (sender_address, Some(sender_account)),
            receiver_address,
            vec![],
        );

        for farmer in farmer_nodes.iter_mut() {
            let _ = farmer.insert_txn_to_mempool(txn.clone());
            let (transaction_kind, validity) = farmer
                .validate_transaction_kind(
                    txn.id(),
                    farmer.mempool_read_handle_factory().clone(),
                    farmer.state_store_read_handle_factory().clone(),
                )
                .unwrap();
            assert!(!validity);
            farmer
                .cast_vote_on_transaction_kind(transaction_kind, validity)
                .unwrap();
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn farmer_node_runtime_can_form_invalid_vote_on_invalid_transaction_sender_missing() {
        let (events_tx, _rx) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);

        let mut nodes = create_node_runtime_network(4, events_tx.clone()).await;

        // NOTE: remove bootstrap
        nodes.pop_front().unwrap();

        let mut node_1 = nodes.pop_front().unwrap();
        assert_eq!(node_1.config.node_type, NodeType::Validator);

        let mut node_2 = nodes.pop_front().unwrap();
        assert_eq!(node_2.config.node_type, NodeType::Validator);

        let node_1_peer_data = PeerData {
            node_id: node_1.config.id.clone(),
            node_type: node_1.config.node_type,
            kademlia_peer_id: node_1.config.kademlia_peer_id.unwrap(),
            udp_gossip_addr: node_1.config.udp_gossip_address,
            raptorq_gossip_addr: node_1.config.raptorq_gossip_address,
            kademlia_liveness_addr: node_1.config.kademlia_liveness_address,
            validator_public_key: node_1.config.keypair.validator_public_key_owned(),
        };

        let node_2_peer_data = PeerData {
            node_id: node_2.config.id.clone(),
            node_type: node_2.config.node_type,
            kademlia_peer_id: node_2.config.kademlia_peer_id.unwrap(),
            udp_gossip_addr: node_2.config.udp_gossip_address,
            raptorq_gossip_addr: node_2.config.raptorq_gossip_address,
            kademlia_liveness_addr: node_2.config.kademlia_liveness_address,
            validator_public_key: node_2.config.keypair.validator_public_key_owned(),
        };

        node_1
            .handle_node_added_to_peer_list(node_2_peer_data.clone())
            .await
            .unwrap();

        node_2
            .handle_node_added_to_peer_list(node_1_peer_data.clone())
            .await
            .unwrap();

        let assigned_membership_1 = AssignedQuorumMembership {
            quorum_kind: QuorumKind::Farmer,
            node_id: node_1.id.clone(),
            pub_key: node_1.config.keypair.validator_public_key_owned(),
            kademlia_peer_id: node_1.config.kademlia_peer_id.unwrap(),
            peers: vec![node_2_peer_data],
        };

        node_1
            .handle_quorum_membership_assigment_created(assigned_membership_1)
            .unwrap();

        let assigned_membership_2 = AssignedQuorumMembership {
            quorum_kind: QuorumKind::Farmer,
            node_id: node_2.id.clone(),
            pub_key: node_2.config.keypair.validator_public_key_owned(),
            kademlia_peer_id: node_2.config.kademlia_peer_id.unwrap(),
            peers: vec![node_1_peer_data],
        };

        node_2
            .handle_quorum_membership_assigment_created(assigned_membership_2)
            .unwrap();

        let mut farmer_nodes = vec![&mut node_1, &mut node_2];

        let mut node_0 = nodes.pop_front().unwrap();

        let (_, sender_public_key) = generate_account_keypair();
        let mut sender_account = Account::new(sender_public_key.into());
        let update_field = AccountField::Credits(100000);
        let _ = sender_account.update_field(update_field);
        let sender_address = node_0.create_account(sender_public_key).unwrap();

        let (_, receiver_public_key) = generate_account_keypair();
        let receiver_account = Account::new(receiver_public_key.into());
        let receiver_address = node_0.create_account(receiver_public_key).unwrap();

        let _sender_account_bytes = bincode::serialize(&sender_account.clone()).unwrap();
        let receiver_account_bytes = bincode::serialize(&receiver_account.clone()).unwrap();

        for farmer in farmer_nodes.iter_mut() {
            let _ = farmer.handle_create_account_requested(
                receiver_address.clone(),
                receiver_account_bytes.clone(),
            );
        }

        let txn = create_txn_from_accounts(
            (sender_address, Some(sender_account)),
            receiver_address,
            vec![],
        );

        for farmer in farmer_nodes.iter_mut() {
            let _ = farmer.insert_txn_to_mempool(txn.clone());
            let (transaction_kind, validity) = farmer
                .validate_transaction_kind(
                    txn.id(),
                    farmer.mempool_read_handle_factory().clone(),
                    farmer.state_store_read_handle_factory().clone(),
                )
                .unwrap();
            assert!(!validity);
            farmer
                .cast_vote_on_transaction_kind(transaction_kind, validity)
                .unwrap();
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn harvesters_can_stash_farmer_votes() {
        let (events_tx, _rx) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);
        let nodes = create_quorum_assigned_node_runtime_network(8, 3, events_tx.clone()).await;

        let mut farmers: Vec<NodeRuntime> = nodes
            .clone()
            .into_iter()
            .filter_map(|nr| {
                if nr.consensus_driver.quorum_kind == Some(QuorumKind::Farmer) {
                    Some(nr)
                } else {
                    None
                }
            })
            .collect();

        let mut harvesters: Vec<NodeRuntime> = nodes
            .into_iter()
            .filter_map(|nr| {
                if nr.consensus_driver.quorum_kind == Some(QuorumKind::Harvester) {
                    Some(nr)
                } else {
                    None
                }
            })
            .collect();

        let ((mut sender_account, sender_address), receiver_address) =
            create_sender_receiver_addresses();

        let update_field = AccountField::Credits(100000);
        let _ = sender_account.update_field(update_field);
        let account_bytes = bincode::serialize(&sender_account.clone()).unwrap();

        let txn = create_txn_from_accounts(
            (sender_address.clone(), Some(sender_account.clone())),
            receiver_address,
            vec![],
        );

        let votes: Vec<Vote> = farmers
            .iter_mut()
            .map(|nr| {
                nr.handle_create_account_requested(sender_address.clone(), account_bytes.clone());
                nr.insert_txn_to_mempool(txn.clone());
                let mempool_reader = nr.mempool_read_handle_factory();
                let state_reader = nr.state_store_read_handle_factory();
                let res = nr
                    .validate_transaction_kind(txn.digest(), mempool_reader, state_reader)
                    .unwrap();
                nr.cast_vote_on_transaction_kind(res.0, res.1).unwrap()
            })
            .collect();

        for harvester in harvesters.iter_mut() {
            let mut res: Result<(), NodeError> = Err(NodeError::Other("".to_string()));
            for vote in &votes {
                res = harvester.handle_vote_received(vote.clone()).await;
                //dbg!(&res);
            }
            assert!(res.is_ok());
        }

        for harvester in harvesters.iter() {
            assert!(
                harvester
                    .consensus_driver
                    .get_quorum_certified_transactions()
                    .len()
                    == 1
            );
        }
    }
}
