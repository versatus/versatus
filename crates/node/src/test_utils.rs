use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, RwLock},
    time::Duration,
};

use async_trait::async_trait;
use block::{Block, BlockHash, ClaimHash, GenesisBlock, InnerBlock, ProposalBlock};
use bulldag::{graph::BullDag, vertex::Vertex};

use crate::{
    data_store::DataStore, network::NetworkEvent, node_runtime::NodeRuntime,
    state_reader::StateReader, Node, NodeError, Result,
};
use events::{AssignedQuorumMembership, EventPublisher, PeerData, DEFAULT_BUFFER};
pub use miner::test_helpers::{create_address, create_claim, create_miner};
use primitives::{
    generate_account_keypair, Address, KademliaPeerId, NodeId, NodeType, QuorumKind, RawSignature,
    Round, Signature,
};
use rand::{seq::SliceRandom, thread_rng};
use secp256k1::{Message, PublicKey, SecretKey};
use signer::engine::SignerEngine;
use storage::vrrbdb::Claims;
use uuid::Uuid;
use vrrb_config::{
    BootstrapQuorumConfig, NodeConfig, NodeConfigBuilder, QuorumMember, QuorumMembershipConfig,
    ThresholdConfig,
};
use vrrb_core::{
    account::{Account, AccountField},
    claim::Claim,
    keypair::Keypair,
    transactions::{
        generate_transfer_digest_vec, NewTransferArgs, QuorumCertifiedTxn, Transaction,
        TransactionDigest, TransactionKind, Transfer,
    },
};
use vrrb_rpc::rpc::{api::RpcApiClient, client::create_client};

pub fn create_mock_full_node_config() -> NodeConfig {
    let data_dir = env::temp_dir();
    let id = Uuid::new_v4().simple().to_string();

    let temp_dir_path = std::env::temp_dir();
    let db_path = temp_dir_path.join(vrrb_core::helpers::generate_random_string());

    let http_api_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let jsonrpc_server_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let rendezvous_local_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let rendezvous_server_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let public_ip_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let udp_gossip_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let raptorq_gossip_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let kademlia_liveness_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);

    NodeConfigBuilder::default()
        .id(id)
        .data_dir(data_dir)
        .db_path(db_path)
        .node_type(NodeType::Bootstrap)
        .bootstrap_config(None)
        .http_api_address(http_api_address)
        .http_api_title(String::from("HTTP Node API"))
        .http_api_version(String::from("1.0"))
        .http_api_shutdown_timeout(Some(Duration::from_secs(5)))
        .jsonrpc_server_address(jsonrpc_server_address)
        .keypair(Keypair::random())
        .rendezvous_local_address(rendezvous_local_address)
        .rendezvous_server_address(rendezvous_server_address)
        .udp_gossip_address(udp_gossip_address)
        .raptorq_gossip_address(raptorq_gossip_address)
        .kademlia_peer_id(Some(KademliaPeerId::rand()))
        .kademlia_liveness_address(kademlia_liveness_address)
        .public_ip_address(public_ip_address)
        .disable_networking(false)
        .quorum_config(None)
        .bootstrap_quorum_config(None)
        .threshold_config(ThresholdConfig::default())
        .whitelisted_nodes(vec![])
        .build()
        .unwrap()
}

#[deprecated]
pub fn create_mock_full_node_config_with_bootstrap(
    bootstrap_node_addresses: Vec<SocketAddr>,
) -> NodeConfig {
    let mut node_config = create_mock_full_node_config();

    node_config
}

#[deprecated]
pub fn create_mock_bootstrap_node_config() -> NodeConfig {
    let mut node_config = create_mock_full_node_config();

    node_config
}

pub fn produce_accounts(n: usize) -> Vec<(Address, Option<Account>)> {
    (0..n)
        .map(|_| {
            let kp = generate_account_keypair();
            let mut account = Some(Account::new(kp.1.clone().into()));
            account
                .as_mut()
                .unwrap()
                .set_credits(1_000_000_000_000_000_000_000_000_000u128);
            (kp.1.clone().into(), account)
        })
        .collect()
}

pub fn produce_random_claims(n: usize) -> HashSet<Claim> {
    (0..n).map(|x| produce_random_claim(x)).collect()
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

fn produce_random_txs(accounts: &Vec<(Address, Option<Account>)>) -> HashSet<TransactionKind> {
    accounts
        .clone()
        .iter()
        .enumerate()
        .map(|(idx, (address, account))| {
            let receiver = if (idx + 1) == accounts.len() {
                accounts[0].clone()
            } else {
                accounts[idx + 1].clone()
            };

            let mut validators: Vec<(String, bool)> = vec![];

            accounts.clone().iter().for_each(|validator| {
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
    mut sig_engine: SignerEngine,
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

            let keypair = Keypair::random();

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
                let edges = edges
                    .iter()
                    .map(|(source, reference)| (source, reference))
                    .collect();

                guard.extend_from_edges(edges);
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

pub fn create_txn_from_accounts_invalid_signature(
    sender: (Address, Option<Account>),
    receiver: Address,
    validators: Vec<(String, bool)>,
) -> TransactionKind {
    let (sk1, pk1) = create_keypair();
    let (sk2, pk2) = create_keypair();
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

/// Creates a blank `block::Certificate` from a `Claim` signature.
pub(crate) fn create_blank_certificate(
    threshold_signature: Vec<(NodeId, Signature)>,
) -> block::Certificate {
    block::Certificate {
        signatures: threshold_signature,
        inauguration: None,
        root_hash: "".to_string(),
        block_hash: "".to_string(),
    }
}

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
    let farmer_count = n * 2 / total_elements;
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

#[derive(Debug, Clone, Default)]
pub struct MockStateStore {}

impl MockStateStore {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Debug, Clone, Default)]
pub struct MockStateReader {}

impl MockStateReader {
    pub fn new() -> Self {
        MockStateReader {}
    }
}

#[async_trait]
impl StateReader for MockStateReader {
    /// Returns a full list of all accounts within state
    async fn state_snapshot(&self) -> Result<HashMap<Address, Account>> {
        todo!()
    }

    /// Returns a full list of transactions pending to be confirmed
    async fn mempool_snapshot(&self) -> Result<HashMap<TransactionDigest, TransactionKind>> {
        todo!()
    }

    /// Get a transaction from state
    async fn get_transaction(
        &self,
        transaction_digest: TransactionDigest,
    ) -> Result<TransactionKind> {
        todo!()
    }

    /// List a group of transactions
    async fn list_transactions(
        &self,
        digests: Vec<TransactionDigest>,
    ) -> Result<HashMap<TransactionDigest, TransactionKind>> {
        todo!()
    }

    async fn get_account(&self, address: Address) -> Result<Account> {
        todo!()
    }

    async fn get_round(&self) -> Result<Round> {
        todo!()
    }

    async fn get_blocks(&self) -> Result<Vec<Block>> {
        todo!()
    }

    async fn get_transaction_count(&self) -> Result<usize> {
        todo!()
    }

    async fn get_claims_by_account_id(&self) -> Result<Vec<Claim>> {
        todo!()
    }

    async fn get_claim_hashes(&self) -> Result<Vec<ClaimHash>> {
        todo!()
    }

    async fn get_claims(&self, claim_hashes: Vec<ClaimHash>) -> Result<Claims> {
        todo!()
    }

    async fn get_last_block(&self) -> Result<Block> {
        todo!()
    }

    fn state_store_values(&self) -> HashMap<Address, Account> {
        todo!()
    }

    /// Returns a copy of all values stored within the state trie
    fn transaction_store_values(&self) -> HashMap<TransactionDigest, TransactionKind> {
        todo!()
    }

    fn claim_store_values(&self) -> HashMap<NodeId, Claim> {
        todo!()
    }
}

#[async_trait]
impl DataStore<MockStateReader> for MockStateStore {
    type Error = NodeError;

    fn state_reader(&self) -> MockStateReader {
        todo!()
    }
}

/// Creates `n` Node instances that make up a network.
pub async fn create_test_network(n: u16) -> Vec<Node> {
    let validator_count = (n as f64 * 0.8).ceil() as usize;
    let miner_count = n as usize - validator_count;

    let mut nodes = vec![];
    let mut quorum_members = BTreeMap::new();

    for i in 1..=n as u16 {
        let udp_port: u16 = 11000 + i;
        let raptor_port: u16 = 12000 + i;
        let kademlia_port: u16 = 13000 + i;

        // let threshold_sk = ValidatorSecretKey::random();
        // let validator_public_key = threshold_sk.public_key();
        let keypair = Keypair::random();
        let validator_public_key = keypair.miner_public_key_owned();

        let node_id = format!("node-{}", i);

        let member = QuorumMember {
            node_id: format!("node-{}", i),
            kademlia_peer_id: KademliaPeerId::rand(),
            node_type: NodeType::Validator,
            udp_gossip_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), udp_port),
            raptorq_gossip_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), raptor_port),
            kademlia_liveness_address: SocketAddr::new(
                IpAddr::V4(Ipv4Addr::LOCALHOST),
                kademlia_port,
            ),
            validator_public_key,
        };

        quorum_members.insert(node_id, member);
    }

    let bootstrap_quorum_config = BootstrapQuorumConfig {
        membership_config: QuorumMembershipConfig {
            quorum_members: quorum_members.clone(),
            quorum_kind: QuorumKind::Farmer,
        },
        genesis_transaction_threshold: (n / 2) as u64,
    };

    let mut config = create_mock_full_node_config();
    config.id = String::from("node-0");

    config.bootstrap_quorum_config = Some(bootstrap_quorum_config.clone());

    let node_0 = Node::start(config).await.unwrap();

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);

    let mut bootstrap_node_config = vrrb_config::BootstrapConfig {
        id: node_0.kademlia_peer_id(),
        udp_gossip_addr: addr,
        raptorq_gossip_addr: addr,
        kademlia_liveness_addr: addr,
    };

    bootstrap_node_config.udp_gossip_addr = node_0.udp_gossip_address();
    bootstrap_node_config.raptorq_gossip_addr = node_0.raptorq_gossip_address();
    bootstrap_node_config.kademlia_liveness_addr = node_0.kademlia_liveness_address();

    nodes.push(node_0);

    for i in 1..=validator_count - 1 {
        let mut config = create_mock_full_node_config();

        let node_id = format!("node-{}", i);
        let quorum_config = quorum_members.get(&node_id).unwrap().to_owned();

        config.id = format!("node-{}", i);
        config.bootstrap_config = Some(bootstrap_node_config.clone());
        config.node_type = NodeType::Validator;
        config.kademlia_liveness_address = quorum_config.kademlia_liveness_address;
        config.raptorq_gossip_address = quorum_config.raptorq_gossip_address;
        config.udp_gossip_address = quorum_config.udp_gossip_address;
        config.kademlia_peer_id = Some(quorum_config.kademlia_peer_id);

        let node = Node::start(config).await.unwrap();
        nodes.push(node);
    }

    for i in validator_count..=validator_count + miner_count {
        let mut miner_config = create_mock_full_node_config();

        let node_id = format!("node-{}", i);
        let quorum_config = quorum_members.get(&node_id).unwrap().to_owned();

        miner_config.id = format!("node-{}", i);
        miner_config.bootstrap_config = Some(bootstrap_node_config.clone());
        miner_config.node_type = NodeType::Miner;
        miner_config.kademlia_liveness_address = quorum_config.kademlia_liveness_address;
        miner_config.raptorq_gossip_address = quorum_config.raptorq_gossip_address;
        miner_config.udp_gossip_address = quorum_config.udp_gossip_address;
        miner_config.kademlia_peer_id = Some(quorum_config.kademlia_peer_id);

        let miner_node = Node::start(miner_config).await.unwrap();

        nodes.push(miner_node);
    }

    nodes
}

/// Creates n NodeRuntimes to simulate networks
pub async fn create_node_runtime_network(
    n: usize,
    events_tx: EventPublisher,
) -> VecDeque<NodeRuntime> {
    let validator_count = (n as f64 * 0.8).ceil() as usize;
    let miner_count = n as usize - validator_count;

    let mut nodes = VecDeque::new();

    let mut quorum_members = BTreeMap::new();

    for i in 1..=n as u16 {
        let udp_port: u16 = 11000 + i;
        let raptor_port: u16 = 12000 + i;
        let kademlia_port: u16 = 13000 + i;
        let keypair = Keypair::random();
        let validator_public_key = keypair.miner_public_key_owned();

        let node_id = format!("node-{}", i);

        let member = QuorumMember {
            node_id: node_id.clone(),
            kademlia_peer_id: KademliaPeerId::rand(),
            node_type: NodeType::Validator,
            udp_gossip_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), udp_port),
            raptorq_gossip_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), raptor_port),
            kademlia_liveness_address: SocketAddr::new(
                IpAddr::V4(Ipv4Addr::LOCALHOST),
                kademlia_port,
            ),
            validator_public_key,
        };

        quorum_members.insert(node_id, member);
    }

    let bootstrap_quorum_config = BootstrapQuorumConfig {
        membership_config: QuorumMembershipConfig {
            quorum_members: quorum_members.clone(),
            quorum_kind: QuorumKind::Farmer,
        },
        genesis_transaction_threshold: (n / 2) as u64,
    };

    let mut config = create_mock_full_node_config();
    config.id = String::from("node-0");

    config.bootstrap_quorum_config = Some(bootstrap_quorum_config.clone());

    let node_0 = NodeRuntime::new(&config, events_tx.clone()).await.unwrap();

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);

    let mut bootstrap_node_config = vrrb_config::BootstrapConfig {
        id: node_0.config.kademlia_peer_id.unwrap(),
        udp_gossip_addr: addr,
        raptorq_gossip_addr: addr,
        kademlia_liveness_addr: addr,
    };

    bootstrap_node_config.udp_gossip_addr = node_0.config.udp_gossip_address;
    bootstrap_node_config.raptorq_gossip_addr = node_0.config.raptorq_gossip_address;
    bootstrap_node_config.kademlia_liveness_addr = node_0.config.kademlia_liveness_address;

    nodes.push_back(node_0);

    for i in 1..=validator_count - 1 {
        let mut config = create_mock_full_node_config();

        let node_id = format!("node-{}", i);
        let quorum_config = quorum_members.get(&node_id).unwrap().to_owned();

        config.id = format!("node-{}", i);
        config.bootstrap_config = Some(bootstrap_node_config.clone());
        config.bootstrap_quorum_config = Some(bootstrap_quorum_config.clone());
        config.node_type = NodeType::Validator;
        config.kademlia_liveness_address = quorum_config.kademlia_liveness_address;
        config.raptorq_gossip_address = quorum_config.raptorq_gossip_address;
        config.udp_gossip_address = quorum_config.udp_gossip_address;
        config.kademlia_peer_id = Some(quorum_config.kademlia_peer_id);

        let node = NodeRuntime::new(&config, events_tx.clone()).await.unwrap();
        nodes.push_back(node);
    }

    for i in validator_count..=validator_count + miner_count {
        let mut miner_config = create_mock_full_node_config();

        let node_id = format!("node-{}", i);
        let quorum_config = quorum_members.get(&node_id).unwrap().to_owned();

        miner_config.id = format!("node-{}", i);
        miner_config.bootstrap_config = Some(bootstrap_node_config.clone());
        miner_config.bootstrap_quorum_config = Some(bootstrap_quorum_config.clone());
        miner_config.node_type = NodeType::Miner;
        miner_config.kademlia_liveness_address = quorum_config.kademlia_liveness_address;
        miner_config.raptorq_gossip_address = quorum_config.raptorq_gossip_address;
        miner_config.udp_gossip_address = quorum_config.udp_gossip_address;
        miner_config.kademlia_peer_id = Some(quorum_config.kademlia_peer_id);

        let miner_node = NodeRuntime::new(&miner_config, events_tx.clone())
            .await
            .unwrap();

        nodes.push_back(miner_node);
    }

    nodes
}

pub async fn create_quorum_assigned_node_runtime_network(
    n: usize,
    min_quorum_size: usize,
    events_tx: EventPublisher,
) -> Vec<NodeRuntime> {
    assert!(n > (1 + (min_quorum_size * 2)));
    let mut nodes = create_node_runtime_network(n, events_tx.clone()).await;
    // NOTE: remove bootstrap
    nodes.pop_front().unwrap();

    let mut quorums = vec![];
    form_groups_with_peer_data(&mut nodes, min_quorum_size, &mut quorums);
    add_group_peer_data_to_node(&mut quorums).await;
    let mut assigned_memberships = vec![];
    assign_node_to_quorum(&quorums, &mut assigned_memberships);
    let mut quorums_only = quorums.into_iter().map(|(nr, _)| nr).collect();
    handle_assigned_memberships(&mut quorums_only, assigned_memberships);
    let flattened = quorums_only.into_iter().flatten().collect();
    flattened
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
    quorums: &Vec<(Vec<NodeRuntime>, Vec<PeerData>)>,
    assigned_memberships: &mut Vec<AssignedQuorumMembership>,
) {
    for (idx, (group, peer_data)) in quorums.into_iter().enumerate() {
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
                assign_node_to_harvester_quorum(
                    &node,
                    assigned_memberships,
                    node_peer_data.clone(),
                );
            } else {
                assign_node_to_farmer_quorum(&node, assigned_memberships, node_peer_data.clone());
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
    let receiver_address = Address::new(receiver_public_key.into());

    ((sender_account, sender_address), receiver_address)
}
