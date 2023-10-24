//! Genesis block should contain a list of value transfer transactions to pre configured addresses. These transactions should allocate a pre configurable number of tokens.
use block::GenesisReceiver;
use events::DEFAULT_BUFFER;
use node::test_utils::create_quorum_assigned_node_runtime_network;
use primitives::{Address, NodeType};
use vrrb_core::transactions::TransactionKind;

/// Genesis blocks created by elected Miner nodes should contain at least one transaction
#[tokio::test]
async fn genesis_block_contains_txns() {
    let (events_tx, _rx) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);
    let mut nodes = create_quorum_assigned_node_runtime_network(8, 3, events_tx.clone()).await;

    let mut genesis_miner = nodes.first_mut().unwrap().clone();
    genesis_miner.config.node_type = NodeType::Miner;
    let receiver_addresses = nodes
        .iter()
        .map(|node| Address::new(node.config.keypair.miner_public_key_owned()))
        .collect::<Vec<Address>>();
    let receivers = assign_genesis_receivers(receiver_addresses);
    let genesis_txns = genesis_miner
        .produce_genesis_transactions(receivers)
        .unwrap();
    let genesis_block = genesis_miner
        .mine_genesis_block(genesis_txns.clone())
        .unwrap();
    assert!(genesis_block.txns.len() >= 1);
}

/// The transactions within the genesis block should be valid and contain balance allocations to at least one address
#[tokio::test]
async fn genesis_block_txns_are_valid() {
    let (events_tx, _rx) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);
    let mut nodes = create_quorum_assigned_node_runtime_network(8, 3, events_tx.clone()).await;

    let mut genesis_miner = nodes.first_mut().unwrap().clone();
    let sender_address = Address::new(genesis_miner.config.keypair.miner_public_key_owned());
    genesis_miner.config.node_type = NodeType::Miner;
    let receiver_addresses = nodes
        .iter()
        .map(|node| Address::new(node.config.keypair.miner_public_key_owned()))
        .collect::<Vec<Address>>();
    let receivers = assign_genesis_receivers(receiver_addresses);
    let genesis_txns = genesis_miner
        .produce_genesis_transactions(receivers)
        .unwrap();
    let genesis_block = genesis_miner
        .mine_genesis_block(genesis_txns.clone())
        .unwrap();
    for (_, txn_kind) in genesis_block.txns.iter() {
        let TransactionKind::Transfer(transfer) = txn_kind;
        {
            assert_eq!(&transfer.sender_address, &sender_address);
            assert!(genesis_txns.contains_key(&transfer.id));
            assert!(transfer.amount > 0);
        }
    }
}

/// All transactions within the genesis block should be applied to the network's state
#[tokio::test]
async fn genesis_block_txns_are_applied_to_state() {
    let (events_tx, _rx) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);
    let mut nodes = create_quorum_assigned_node_runtime_network(8, 3, events_tx.clone()).await;

    let mut genesis_miner = nodes.first_mut().unwrap().clone();
    genesis_miner.config.node_type = NodeType::Miner;
    let receiver_addresses = nodes
        .iter()
        .map(|node| Address::new(node.config.keypair.miner_public_key_owned()))
        .collect::<Vec<Address>>();
    let receivers = assign_genesis_receivers(receiver_addresses);
    let genesis_txns = genesis_miner
        .produce_genesis_transactions(receivers)
        .unwrap();
    let genesis_block = genesis_miner
        .mine_genesis_block(genesis_txns.clone())
        .unwrap();
    nodes.iter_mut().for_each(|node| {
        dbg!(&node.state_snapshot());
    });
}

fn assign_genesis_receivers(receiver_addresses: Vec<Address>) -> Vec<GenesisReceiver> {
    receiver_addresses
        .iter()
        .map(|address| GenesisReceiver::create_contributor(address))
        .collect()
}
