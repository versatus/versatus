//! Genesis block should contain a list of value transfer transactions to pre configured addresses. These transactions should allocate a pre configurable number of tokens.
use block::{Block, Certificate, GenesisReceiver};
use events::DEFAULT_BUFFER;
use node::{
    node_runtime::NodeRuntime, test_utils::create_quorum_assigned_node_runtime_network, NodeError,
};
use primitives::{Address, NodeType, Signature};
use storage::vrrbdb::ApplyBlockResult;

/// Genesis blocks created by elected Miner nodes should contain at least one transaction
#[tokio::test]
async fn genesis_block_contains_txns() {
    let (events_tx, _rx) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);
    let mut nodes = create_quorum_assigned_node_runtime_network(8, 3, events_tx.clone()).await;

    nodes.reverse();
    nodes.pop(); // pop bootstrap node
    let mut genesis_miner = nodes.pop().unwrap();
    genesis_miner.config.node_type = NodeType::Miner;

    let receiver_addresses = nodes
        .iter()
        .map(|node| Address::new(node.config.keypair.miner_public_key_owned()))
        .collect::<Vec<Address>>();
    let receivers = assign_genesis_receivers(receiver_addresses);
    let genesis_rewards = genesis_miner.distribute_genesis_reward(receivers).unwrap();
    let genesis_block = genesis_miner
        .mine_genesis_block(genesis_rewards.clone())
        .unwrap();
    assert!(genesis_block.genesis_rewards.0.len() >= 1);
}

/// The transactions within the genesis block should be valid and contain balance allocations to at least one address
#[tokio::test]
async fn genesis_block_txns_are_valid() {
    let (events_tx, _rx) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);
    let mut nodes = create_quorum_assigned_node_runtime_network(8, 3, events_tx.clone()).await;

    nodes.reverse();
    nodes.pop(); // pop bootstrap node
    let mut genesis_miner = nodes.pop().unwrap();
    genesis_miner.config.node_type = NodeType::Miner;

    let receiver_addresses = nodes
        .iter()
        .map(|node| Address::new(node.config.keypair.miner_public_key_owned()))
        .collect::<Vec<Address>>();
    let receivers = assign_genesis_receivers(receiver_addresses.clone());
    let genesis_rewards = genesis_miner.distribute_genesis_reward(receivers).unwrap();
    let genesis_block = genesis_miner
        .mine_genesis_block(genesis_rewards.clone())
        .unwrap();
    for (genesis_receiver, reward) in genesis_block.genesis_rewards.0.iter() {
        assert!(genesis_rewards.0.contains_key(&genesis_receiver));
        assert!(*reward > 0);
    }
}

#[tokio::test]
async fn genesis_block_can_be_certified() {
    let (events_tx, _rx) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);
    let mut nodes = create_quorum_assigned_node_runtime_network(8, 3, events_tx.clone()).await;

    nodes.reverse();
    nodes.pop(); // pop bootstrap node
    let mut genesis_miner = nodes.pop().unwrap();
    genesis_miner.config.node_type = NodeType::Miner;

    let receiver_addresses = nodes
        .iter()
        .map(|node| Address::new(node.config.keypair.miner_public_key_owned()))
        .collect::<Vec<Address>>();
    let receivers = assign_genesis_receivers(receiver_addresses.clone());
    let genesis_rewards = genesis_miner.distribute_genesis_reward(receivers).unwrap();
    let genesis_block = genesis_miner
        .mine_genesis_block(genesis_rewards.clone())
        .unwrap();

    let mut harvesters: Vec<NodeRuntime> = nodes
        .iter()
        .filter_map(|node| {
            if node.consensus_driver.is_harvester().is_ok() {
                Some(node.clone())
            } else {
                None
            }
        })
        .collect();
    let mut chosen_harvester = harvesters.pop().unwrap();
    let _ = chosen_harvester.state_driver.append_genesis(&genesis_block);
    let mut sigs: Vec<Signature> = Vec::new();
    for node in harvesters.iter_mut() {
        sigs.push(
            node.handle_sign_block(Block::Genesis {
                block: genesis_block.clone(),
            })
            .await
            .unwrap(),
        );
        let _ = node.state_driver.append_genesis(&genesis_block);
    }
    let mut res: Result<Certificate, NodeError> = Err(NodeError::Other("".to_string()));
    for (sig, harvester) in sigs.into_iter().zip(harvesters.iter()) {
        res = chosen_harvester.certify_genesis_block(genesis_block.clone());
    }
    assert!(res.is_ok());
}

#[tokio::test]
async fn certified_genesis_block_appended_to_dag() {
    let (events_tx, _rx) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);
    let mut nodes = create_quorum_assigned_node_runtime_network(8, 3, events_tx.clone()).await;

    nodes.reverse();
    nodes.pop(); // pop bootstrap node
    let mut genesis_miner = nodes.pop().unwrap();
    genesis_miner.config.node_type = NodeType::Miner;

    let receiver_addresses = nodes
        .iter()
        .map(|node| Address::new(node.config.keypair.miner_public_key_owned()))
        .collect::<Vec<Address>>();
    let receivers = assign_genesis_receivers(receiver_addresses.clone());
    let genesis_rewards = genesis_miner.distribute_genesis_reward(receivers).unwrap();
    let genesis_block = genesis_miner
        .mine_genesis_block(genesis_rewards.clone())
        .unwrap();

    let mut harvesters: Vec<NodeRuntime> = nodes
        .iter()
        .filter_map(|node| {
            if node.consensus_driver.is_harvester().is_ok() {
                Some(node.clone())
            } else {
                None
            }
        })
        .collect();
    let harvester = harvesters.first_mut().unwrap();
    let certificate = harvester.certify_genesis_block(genesis_block).unwrap();
    // append cert to dag
}

/// All transactions within the genesis block should be applied to the network's state
#[tokio::test]
async fn genesis_block_txns_are_applied_to_state() {
    let (events_tx, _rx) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);
    let mut nodes = create_quorum_assigned_node_runtime_network(8, 3, events_tx.clone()).await;

    nodes.reverse();
    nodes.pop(); // pop bootstrap node
    let mut genesis_miner = nodes.pop().unwrap();
    genesis_miner.config.node_type = NodeType::Miner;
    let mut all_nodes: Vec<NodeRuntime> = nodes
        .iter()
        .filter_map(|node| {
            if !node.consensus_driver.is_bootstrap_node() {
                Some(node.clone())
            } else {
                None
            }
        })
        .collect();

    let receiver_addresses = nodes
        .iter()
        .map(|node| Address::new(node.config.keypair.miner_public_key_owned()))
        .collect::<Vec<Address>>();
    let receivers = assign_genesis_receivers(receiver_addresses);
    let genesis_reward_state_updates = genesis_miner.distribute_genesis_reward(receivers).unwrap();
    let genesis_block = genesis_miner
        .mine_genesis_block(genesis_reward_state_updates.clone())
        .unwrap();
    // apply rewards
    let results: Vec<ApplyBlockResult> = all_nodes
        .iter_mut()
        .map(|node| {
            node.handle_block_received(block::Block::Genesis {
                block: genesis_block.clone().into(),
            })
            .unwrap()
        })
        .collect();
    let apply_block_result = results.first().unwrap();

    results.iter().for_each(|res| {
        assert_eq!(
            res.transactions_root_hash_str(),
            apply_block_result.transactions_root_hash_str()
        );
        assert_eq!(
            res.state_root_hash_str(),
            apply_block_result.state_root_hash_str()
        );
    });
}

fn assign_genesis_receivers(receiver_addresses: Vec<Address>) -> Vec<GenesisReceiver> {
    receiver_addresses
        .iter()
        .map(|address| GenesisReceiver::new(address.clone()))
        .collect()
}
