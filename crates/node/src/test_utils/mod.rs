use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet, VecDeque},
    hash::{Hash, Hasher},
    net::SocketAddr,
    sync::{Arc, RwLock},
};

use block::{
    header::BlockHeader, Block, BlockHash, ConvergenceBlock, GenesisBlock, InnerBlock,
    ProposalBlock,
};
use bulldag::{graph::BullDag, vertex::Vertex};
use quorum::{election::Election, quorum::Quorum};

use crate::{network::NetworkEvent, node_runtime::NodeRuntime, Result};
use events::{AssignedQuorumMembership, PeerData, DEFAULT_BUFFER};
pub use miner::test_helpers::{create_address, create_claim, create_miner};
pub use mock_config::*;
pub use node_network::*;
use primitives::{generate_account_keypair, Address, NodeId, NodeType, QuorumKind};
use rand::{seq::SliceRandom, thread_rng};
pub use runtime_network::*;
use secp256k1::{Message, PublicKey, SecretKey};
use sha256::digest;
use signer::engine::SignerEngine;

use vrrb_config::QuorumMember;
use vrrb_core::{
    account::{Account, AccountField},
    claim::Claim,
    keypair::{KeyPair, Keypair},
    transactions::{
        generate_transfer_digest_vec, NewTransferArgs, Transaction, TransactionDigest,
        TransactionKind, Transfer,
    },
};
use vrrb_rpc::rpc::{api::RpcApiClient, client::create_client};

mod mock_config;
mod node_network;
mod runtime_network;

pub(crate) const INITIAL_ACCOUNT_CREDITS: u128 = 1_000_000_000_000_000_000_000_000_000;

pub fn produce_accounts(n: usize) -> Vec<(Address, Option<Account>)> {
    (0..n)
        .map(|_| {
            let kp = generate_account_keypair();
            let mut account = Some(Account::new(kp.1.into()));
            account
                .as_mut()
                .unwrap()
                .set_credits(INITIAL_ACCOUNT_CREDITS);
            (kp.1.into(), account)
        })
        .collect()
}

pub fn produce_random_claims(n: usize) -> HashSet<Claim> {
    (0..n).map(produce_random_claim).collect()
}

pub fn produce_random_claim(x: usize) -> Claim {
    let kp = Keypair::random();
    let address = Address::new(kp.miner_kp.1);
    let ip_address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    let signature = Claim::signature_for_valid_claim(
        kp.miner_kp.1,
        ip_address,
        kp.get_miner_secret_key().secret_bytes().to_vec(),
    )
    .unwrap();

    Claim::new(
        kp.miner_kp.1,
        address,
        ip_address,
        signature,
        format!("node-{x}"),
    )
    .unwrap()
}

fn produce_random_txs(accounts: &[(Address, Option<Account>)]) -> HashSet<TransactionKind> {
    accounts
        .iter()
        .enumerate()
        .map(|(idx, (address, account))| {
            let receiver = if (idx + 1) == accounts.len() {
                accounts[0].clone()
            } else {
                accounts[idx + 1].clone()
            };

            let mut validators: Vec<(String, bool)> = vec![];

            accounts.iter().for_each(|validator| {
                if (validator.clone() != receiver)
                    && (validator.clone() != (address.clone(), account.clone()))
                {
                    let pk = validator.clone().0.to_string();
                    validators.push((pk, true));
                }
            });
            create_txn_from_accounts((address.clone(), account.clone()), receiver.0, validators)
        })
        .collect()
}

pub fn produce_genesis_block() -> GenesisBlock {
    let genesis = miner::test_helpers::mine_genesis();
    genesis.unwrap()
}

pub fn produce_proposal_blocks(
    last_block_hash: BlockHash,
    accounts: Vec<(Address, Option<Account>)>,
    n: usize,
    ntx: usize,
    sig_engine: SignerEngine,
) -> Vec<ProposalBlock> {
    (0..n)
        .map(|_| {
            let kp = Keypair::random();
            let address = Address::new(kp.miner_kp.1);
            let ip_address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
            let signature = Claim::signature_for_valid_claim(
                kp.miner_kp.1,
                ip_address,
                kp.get_miner_secret_key().secret_bytes().to_vec(),
            )
            .unwrap();

            let from = Claim::new(
                kp.miner_kp.1,
                address,
                ip_address,
                signature,
                NodeId::default(),
            )
            .unwrap();
            let txs = produce_random_txs(&accounts);
            let claims = produce_random_claims(ntx);

            let txn_list = txs
                .into_iter()
                .map(|txn| {
                    let digest = txn.id();
                    (digest, txn.clone())
                })
                .collect();

            let claim_list = claims
                .into_iter()
                .map(|claim| (claim.hash, claim))
                .collect();

            let _keypair = Keypair::random();

            ProposalBlock::build(
                last_block_hash.clone(),
                0,
                0,
                txn_list,
                claim_list,
                from,
                sig_engine.clone(),
            )
        })
        .collect()
}

pub fn produce_convergence_block(dag: Arc<RwLock<BullDag<Block, BlockHash>>>) -> Option<BlockHash> {
    let keypair = Keypair::random();
    let mut miner = miner::test_helpers::create_miner_from_keypair(&keypair);
    miner.dag = dag.clone();
    let last_block = miner::test_helpers::get_genesis_block_from_dag(dag.clone());

    if let Some(block) = last_block {
        miner.last_block = Some(Arc::new(block));
    }

    if let Ok(cblock) = miner.try_mine() {
        if let Block::Convergence { ref block } = cblock.clone() {
            let cvtx: Vertex<Block, String> = cblock.into();
            let mut edges: Vec<(Vertex<Block, String>, Vertex<Block, String>)> = vec![];
            if let Ok(guard) = dag.read() {
                block.clone().get_ref_hashes().iter().for_each(|t| {
                    if let Some(pvtx) = guard.get_vertex(t.clone()) {
                        edges.push((pvtx.clone(), cvtx.clone()));
                    }
                });
            }

            if let Ok(mut guard) = dag.write() {
                let mut ext_edges = Vec::with_capacity(edges.len());
                for (source, reference) in &edges {
                    ext_edges.push((source, reference));
                }

                guard.extend_from_edges(&ext_edges);
                return Some(block.get_hash());
            }
        }
    }

    None
}

pub fn create_keypair() -> (SecretKey, PublicKey) {
    let kp = Keypair::random();
    kp.miner_kp
}

pub fn create_txn_from_accounts(
    sender: (Address, Option<Account>),
    receiver: Address,
    validators: Vec<(String, bool)>,
) -> TransactionKind {
    let (sk, pk) = create_keypair();
    let saddr = sender.0.clone();
    let raddr = receiver;
    let amount = 100u128.pow(2);
    let token = None;

    let validators = validators
        .iter()
        .map(|(k, v)| (k.to_string(), *v))
        .collect();

    let txn_args = NewTransferArgs {
        timestamp: chrono::Utc::now().timestamp(),
        sender_address: saddr,
        sender_public_key: pk,
        receiver_address: raddr,
        token,
        amount,
        signature: sk
            .sign_ecdsa(Message::from_hashed_data::<secp256k1::hashes::sha256::Hash>(b"vrrb")),
        validators: Some(validators),
        nonce: sender.1.unwrap().nonce() + 1,
    };

    let mut txn = TransactionKind::Transfer(Transfer::new(txn_args));

    txn.sign(&sk);

    let txn_digest_vec = generate_transfer_digest_vec(
        txn.timestamp(),
        txn.sender_address().to_string(),
        txn.sender_public_key(),
        txn.receiver_address().to_string(),
        txn.token().clone(),
        txn.amount(),
        txn.nonce(),
    );

    let _digest = TransactionDigest::from(txn_digest_vec);

    txn
}

//TODO: sk1 & pk2 are not being used.
pub fn create_txn_from_accounts_invalid_signature(
    sender: (Address, Option<Account>),
    receiver: Address,
    validators: Vec<(String, bool)>,
) -> TransactionKind {
    let (_sk1, pk1) = create_keypair();
    let (sk2, _pk2) = create_keypair();
    let saddr = sender.0.clone();
    let raddr = receiver;
    let amount = 100u128.pow(2);
    let token = None;

    let validators = validators
        .iter()
        .map(|(k, v)| (k.to_string(), *v))
        .collect();

    let txn_args = NewTransferArgs {
        timestamp: chrono::Utc::now().timestamp(),
        sender_address: saddr,
        sender_public_key: pk1,
        receiver_address: raddr,
        token,
        amount,
        signature: sk2
            .sign_ecdsa(Message::from_hashed_data::<secp256k1::hashes::sha256::Hash>(b"vrrb")),
        validators: Some(validators),
        nonce: sender.1.unwrap().nonce() + 1,
    };

    let mut txn = TransactionKind::Transfer(Transfer::new(txn_args));

    txn.sign(&sk2);

    let txn_digest_vec = generate_transfer_digest_vec(
        txn.timestamp(),
        txn.sender_address().to_string(),
        txn.sender_public_key(),
        txn.receiver_address().to_string(),
        txn.token().clone(),
        txn.amount(),
        txn.nonce(),
    );

    let _digest = TransactionDigest::from(txn_digest_vec);

    txn
}

pub fn create_txn_from_accounts_invalid_timestamp(
    sender: (Address, Option<Account>),
    receiver: Address,
    validators: Vec<(String, bool)>,
) -> TransactionKind {
    let (sk, pk) = create_keypair();
    let saddr = sender.0.clone();
    let raddr = receiver;
    let amount = 100u128.pow(2);
    let token = None;

    let validators = validators
        .iter()
        .map(|(k, v)| (k.to_string(), *v))
        .collect();

    let txn_args = NewTransferArgs {
        timestamp: 0,
        sender_address: saddr,
        sender_public_key: pk,
        receiver_address: raddr,
        token,
        amount,
        signature: sk
            .sign_ecdsa(Message::from_hashed_data::<secp256k1::hashes::sha256::Hash>(b"vrrb")),
        validators: Some(validators),
        nonce: sender.1.unwrap().nonce() + 1,
    };

    let mut txn = TransactionKind::Transfer(Transfer::new(txn_args));

    txn.sign(&sk);

    let txn_digest_vec = generate_transfer_digest_vec(
        txn.timestamp(),
        txn.sender_address().to_string(),
        txn.sender_public_key(),
        txn.receiver_address().to_string(),
        txn.token().clone(),
        txn.amount(),
        txn.nonce(),
    );

    let _digest = TransactionDigest::from(txn_digest_vec);

    txn
}

// /// Creates a `DagModule` for testing the event handler.
// pub(crate) fn create_dag_module() -> DagModule {
//     let miner = create_miner();
//     let (sk, pk) = create_keypair();
//     let addr = create_address(&pk);
//     let ip_address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
//     let signature =
//         Claim::signature_for_valid_claim(pk, ip_address, sk.secret_bytes().to_vec()).unwrap();
//
//     let claim = create_claim(&pk, &addr, ip_address, signature);
//     let (events_tx, _) = tokio::sync::mpsc::channel(events::DEFAULT_BUFFER);
//
//     DagModule::new(miner.dag, events_tx, claim)
// }

pub async fn create_dyswarm_client(addr: SocketAddr) -> Result<dyswarm::client::Client> {
    let client_config = dyswarm::client::Config { addr };
    let client = dyswarm::client::Client::new(client_config).await?;

    Ok(client)
}

pub async fn send_data_over_quic(data: String, addr: SocketAddr) -> Result<()> {
    let client = create_dyswarm_client(addr).await?;

    let msg = dyswarm::types::Message {
        id: dyswarm::types::MessageId::new_v4(),
        timestamp: 0i64,
        data: NetworkEvent::Ping(data),
    };

    client.send_data_via_quic(msg, addr).await?;

    Ok(())
}

pub fn generate_nodes_pattern(n: usize) -> Vec<NodeType> {
    let total_elements = 8; // Sum of occurrences: 2 + 2 + 4
    let harvester_count = n * 2 / total_elements;
    let miner_count = n * 4 / total_elements;

    let mut array = Vec::with_capacity(n);
    for _ in 0..harvester_count {
        array.push(NodeType::Validator);
    }
    for _ in 0..miner_count {
        array.push(NodeType::Miner);
    }

    array.shuffle(&mut thread_rng());

    array
}

/// Creates an instance of a RpcApiClient for testing.
pub async fn create_node_rpc_client(rpc_addr: SocketAddr) -> impl RpcApiClient {
    create_client(rpc_addr).await.unwrap()
}

/// Creates a mock `NewTxnArgs` struct meant to be used for testing.
pub fn create_mock_transaction_args(n: usize) -> NewTransferArgs {
    let (sk, pk) = create_keypair();
    let (_, rpk) = create_keypair();
    let saddr = create_address(&pk);
    let raddr = create_address(&rpk);
    let amount = (n.pow(2)) as u128;
    let token = None;

    NewTransferArgs {
        timestamp: 0,
        sender_address: saddr,
        sender_public_key: pk,
        receiver_address: raddr,
        token,
        amount,
        signature: sk
            .sign_ecdsa(Message::from_hashed_data::<secp256k1::hashes::sha256::Hash>(b"vrrb")),
        validators: None,
        nonce: n as u128,
    }
}

fn handle_assigned_memberships(
    quorums: &mut Vec<Vec<NodeRuntime>>,
    assigned_memberships: Vec<AssignedQuorumMembership>,
) {
    for group in quorums {
        for node in group {
            node.handle_quorum_membership_assigments_created(assigned_memberships.clone())
                .unwrap();
        }
    }
}

fn assign_node_to_quorum(
    quorums: &[(Vec<NodeRuntime>, Vec<PeerData>)],
    assigned_memberships: &mut Vec<AssignedQuorumMembership>,
) {
    for (idx, (group, peer_data)) in quorums.iter().enumerate() {
        for node in group.iter() {
            let node_peer_data: Vec<PeerData> = peer_data
                .clone()
                .iter()
                .filter_map(|data| {
                    if data.node_id.clone() != node.config.id.clone() {
                        Some(data.clone())
                    } else {
                        None
                    }
                })
                .collect();

            if idx == 0 {
                //dbg!("calling assign node to harvester");
                assign_node_to_harvester_quorum(node, assigned_memberships, node_peer_data.clone());
            } else {
                assign_node_to_farmer_quorum(node, assigned_memberships, node_peer_data.clone());
            }
        }
    }
}

fn assign_node_to_farmer_quorum(
    node: &NodeRuntime,
    assigned_memberships: &mut Vec<AssignedQuorumMembership>,
    peers: Vec<PeerData>,
) {
    assigned_memberships.push(AssignedQuorumMembership {
        quorum_kind: QuorumKind::Farmer,
        node_id: node.config.id.clone(),
        pub_key: node.config.keypair.validator_public_key_owned(),
        kademlia_peer_id: node.config.kademlia_peer_id.unwrap(),
        peers: peers.clone(),
    });
}

fn assign_node_to_harvester_quorum(
    node: &NodeRuntime,
    assigned_memberships: &mut Vec<AssignedQuorumMembership>,
    peers: Vec<PeerData>,
) {
    assigned_memberships.push(AssignedQuorumMembership {
        quorum_kind: QuorumKind::Harvester,
        node_id: node.config.id.clone(),
        pub_key: node.config.keypair.validator_public_key_owned(),
        kademlia_peer_id: node.config.kademlia_peer_id.unwrap(),
        peers: peers.clone(),
    });
}

async fn add_group_peer_data_to_node(quorums: &mut Vec<(Vec<NodeRuntime>, Vec<PeerData>)>) {
    for (group, group_peer_data) in quorums {
        for node in group.iter_mut() {
            for peer_data in group_peer_data.iter_mut() {
                if peer_data.node_id != node.config.id {
                    node.handle_node_added_to_peer_list(peer_data.clone())
                        .await
                        .unwrap();
                }
            }
        }
    }
}

fn form_groups_with_peer_data(
    nodes: &mut VecDeque<NodeRuntime>,
    min_quorum_size: usize,
    quorums: &mut Vec<(Vec<NodeRuntime>, Vec<PeerData>)>,
) -> Vec<(Vec<NodeRuntime>, Vec<PeerData>)> {
    while nodes.len() >= min_quorum_size {
        let mut group = vec![];
        let mut group_peer_data = vec![];
        while group.len() < min_quorum_size {
            let member = nodes.pop_front().unwrap();
            let peer_data = PeerData {
                node_id: member.config.id.clone(),
                node_type: member.config.node_type,
                kademlia_peer_id: member.config.kademlia_peer_id.unwrap(),
                udp_gossip_addr: member.config.udp_gossip_address,
                raptorq_gossip_addr: member.config.raptorq_gossip_address,
                kademlia_liveness_addr: member.config.kademlia_liveness_address,
                validator_public_key: member.config.keypair.validator_public_key_owned(),
            };

            group.push(member);
            group_peer_data.push(peer_data);
        }
        quorums.push((group, group_peer_data));
    }

    quorums.clone()
}

pub fn create_sender_receiver_addresses() -> ((Account, Address), Address) {
    let (_, sender_public_key) = generate_account_keypair();
    let mut sender_account = Account::new(sender_public_key.into());
    let update_field = AccountField::Credits(100000);
    let _ = sender_account.update_field(update_field);
    let sender_address = Address::new(sender_public_key);

    let (_, receiver_public_key) = generate_account_keypair();
    let receiver_address = Address::new(receiver_public_key);

    ((sender_account, sender_address), receiver_address)
}

pub async fn setup_network(
    n: usize,
) -> (
    NodeRuntime,
    HashMap<NodeId, NodeRuntime>, // farmers
    HashMap<NodeId, NodeRuntime>, // validators
    HashMap<NodeId, NodeRuntime>, // Miners
) {
    let (events_tx, _events_rx) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);

    let mut nodes = create_node_runtime_network(n, events_tx.clone()).await;

    let mut node_0 = nodes.pop_front().unwrap();

    node_0
        .create_account(node_0.config_ref().keypair.miner_public_key_owned())
        .unwrap();

    let mut quorum_assignments = HashMap::new();

    for node in nodes.iter() {
        let peer_data = PeerData {
            node_id: node.config.id.clone(),
            node_type: node.config.node_type,
            kademlia_peer_id: node.config.kademlia_peer_id.unwrap(),
            udp_gossip_addr: node.config.udp_gossip_address,
            raptorq_gossip_addr: node.config.raptorq_gossip_address,
            kademlia_liveness_addr: node.config.kademlia_liveness_address,
            validator_public_key: node.config.keypair.validator_public_key_owned(),
        };

        let assignments = node_0
            .handle_node_added_to_peer_list(peer_data.clone())
            .await
            .unwrap();

        if let Some(assignments) = assignments {
            quorum_assignments.extend(assignments);
        }
    }

    let other_nodes_copy = nodes.clone();

    // NOTE: let nodes be aware of each other
    for node in nodes.iter_mut() {
        for other_node in other_nodes_copy.iter() {
            if node.config.id == other_node.config.id {
                continue;
            }

            let peer_data = PeerData {
                node_id: other_node.config.id.clone(),
                node_type: other_node.config.node_type,
                kademlia_peer_id: other_node.config.kademlia_peer_id.unwrap(),
                udp_gossip_addr: other_node.config.udp_gossip_address,
                raptorq_gossip_addr: other_node.config.raptorq_gossip_address,
                kademlia_liveness_addr: other_node.config.kademlia_liveness_address,
                validator_public_key: other_node.config.keypair.validator_public_key_owned(),
            };

            node.handle_node_added_to_peer_list(peer_data.clone())
                .await
                .unwrap();
        }
    }

    let node_0_pubkey = node_0.config_ref().keypair.miner_public_key_owned();

    // NOTE: create te bootstrap's node account as well as their accounts on everyone's state
    for current_node in nodes.iter_mut() {
        for node in other_nodes_copy.iter() {
            let node_pubkey = node.config_ref().keypair.miner_public_key_owned();
            node_0.create_account(node_pubkey).unwrap();
            current_node.create_account(node_0_pubkey).unwrap();
            current_node.create_account(node_pubkey).unwrap();
        }
    }

    let mut nodes = nodes
        .into_iter()
        .map(|node| (node.config.id.clone(), node))
        .collect::<HashMap<NodeId, NodeRuntime>>();

    for (_node_id, node) in nodes.iter_mut() {
        node.handle_quorum_membership_assigments_created(
            quorum_assignments.clone().into_values().collect(),
        )
        .unwrap();
    }

    let validator_nodes = nodes
        .clone()
        .into_iter()
        .filter(|(_, node)| node.config.node_type == NodeType::Validator)
        .collect::<HashMap<NodeId, NodeRuntime>>();

    let farmer_nodes = validator_nodes
        .clone()
        .into_iter()
        .filter(|(_, node)| node.quorum_membership().unwrap().quorum_kind == QuorumKind::Farmer)
        .collect::<HashMap<NodeId, NodeRuntime>>();

    let harvester_nodes = validator_nodes
        .clone()
        .into_iter()
        .filter(|(_, node)| node.quorum_membership().unwrap().quorum_kind == QuorumKind::Harvester)
        .collect::<HashMap<NodeId, NodeRuntime>>();

    let miner_nodes = nodes
        .clone()
        .into_iter()
        .filter(|(_, node)| node.config.node_type == NodeType::Miner)
        .collect::<HashMap<NodeId, NodeRuntime>>();

    (node_0, farmer_nodes, harvester_nodes, miner_nodes)
}

pub fn dummy_convergence_block() -> ConvergenceBlock {
    let keypair = KeyPair::random();
    let public_key = keypair.get_miner_public_key();
    let mut hasher = DefaultHasher::new();

    let secret_key = keypair.get_miner_secret_key();
    let message =
        Message::from_hashed_data::<secp256k1::hashes::sha256::Hash>("Dummy block".as_bytes());

    let signature = secret_key.sign_ecdsa(message);

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
            miner_signature: signature,
        },
        txns: Default::default(),
        claims: Default::default(),
        hash: "dummy_convergence_block".into(),
        certificate: None,
    }
}

//TODO: account1.update_field & account2.update_field are not being used.
pub fn dummy_proposal_block(sig_engine: signer::engine::SignerEngine) -> ProposalBlock {
    let kp1 = Keypair::random();
    let address1 = Address::new(kp1.miner_kp.1);
    let kp2 = Keypair::random();
    let address2 = Address::new(kp2.miner_kp.1);
    let mut account1 = Account::new(address1.clone());
    let update_field = AccountField::Credits(100000);
    let _ = account1.update_field(update_field.clone());
    let mut account2 = Account::new(address2.clone());
    let _ = account2.update_field(update_field.clone());
    produce_proposal_blocks(
        "dummy_proposal_block".to_string(),
        vec![(address1, Some(account1)), (address2, Some(account2))],
        1,
        2,
        sig_engine,
    )
    .pop()
    .unwrap()
}

//TODO: account1.update_field & account2.update_field are not being used.
pub fn dummy_proposal_block_and_accounts(
    sig_engine: signer::engine::SignerEngine,
) -> ((Address, Account), (Address, Account), ProposalBlock) {
    let kp1 = Keypair::random();
    let address1 = Address::new(kp1.miner_kp.1);
    let kp2 = Keypair::random();
    let address2 = Address::new(kp2.miner_kp.1);
    let mut account1 = Account::new(address1.clone());
    let update_field = AccountField::Credits(100000);
    let _ = account1.update_field(update_field.clone());
    let mut account2 = Account::new(address2.clone());
    let _ = account2.update_field(update_field.clone());
    let proposal_block = produce_proposal_blocks(
        "dummy_proposal_block".to_string(),
        vec![
            (address1.clone(), Some(account1.clone())),
            (address2.clone(), Some(account2.clone())),
        ],
        1,
        1,
        sig_engine,
    )
    .pop()
    .unwrap();

    ((address1, account1), (address2, account2), proposal_block)
}

pub fn setup_whitelisted_nodes(
    farmers: &HashMap<NodeId, NodeRuntime>,
    harvesters: &HashMap<NodeId, NodeRuntime>,
    miners: &HashMap<NodeId, NodeRuntime>,
) -> Vec<QuorumMember> {
    let whitelisted_harvesters = harvesters
        .iter()
        .map(|(_, node)| QuorumMember {
            node_id: node.config.id.clone(),
            kademlia_peer_id: node.config.kademlia_peer_id.unwrap(),
            node_type: node.config.node_type,
            udp_gossip_address: node.config.udp_gossip_address,
            raptorq_gossip_address: node.config.raptorq_gossip_address,
            kademlia_liveness_address: node.config.kademlia_liveness_address,
            validator_public_key: node.config.keypair.miner_public_key_owned(),
            quorum_kind: QuorumKind::Harvester,
        })
        .collect::<Vec<QuorumMember>>();

    let whitelisted_farmers = farmers
        .iter()
        .map(|(_, node)| QuorumMember {
            node_id: node.config.id.clone(),
            kademlia_peer_id: node.config.kademlia_peer_id.unwrap(),
            node_type: node.config.node_type,
            udp_gossip_address: node.config.udp_gossip_address,
            raptorq_gossip_address: node.config.raptorq_gossip_address,
            kademlia_liveness_address: node.config.kademlia_liveness_address,
            validator_public_key: node.config.keypair.miner_public_key_owned(),
            quorum_kind: QuorumKind::Farmer,
        })
        .collect::<Vec<QuorumMember>>();

    let whitelisted_miners = miners
        .iter()
        .map(|(_, node)| QuorumMember {
            node_id: node.config.id.clone(),
            kademlia_peer_id: node.config.kademlia_peer_id.unwrap(),
            node_type: node.config.node_type,
            udp_gossip_address: node.config.udp_gossip_address,
            raptorq_gossip_address: node.config.raptorq_gossip_address,
            kademlia_liveness_address: node.config.kademlia_liveness_address,
            validator_public_key: node.config.keypair.miner_public_key_owned(),
            // TODO: figure out how to refactor this l8r, there's no miner quorum
            quorum_kind: QuorumKind::Miner,
        })
        .collect::<Vec<QuorumMember>>();

    let mut whitelisted_nodes = Vec::new();
    whitelisted_nodes.extend(whitelisted_harvesters);
    whitelisted_nodes.extend(whitelisted_farmers);
    whitelisted_nodes.extend(whitelisted_miners);
    whitelisted_nodes
}
