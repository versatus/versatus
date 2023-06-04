use std::{
    collections::HashSet,
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, RwLock},
    time::Duration,
};

use block::{Block, BlockHash, GenesisBlock, InnerBlock, ProposalBlock};
use bulldag::{graph::BullDag, vertex::Vertex};
use primitives::{generate_account_keypair, Address, NodeType, RawSignature};
use secp256k1::{Message, PublicKey, SecretKey};
use uuid::Uuid;
use vrrb_config::{NodeConfig, NodeConfigBuilder};
use vrrb_core::{
    account::Account,
    claim::Claim,
    keypair::Keypair,
    txn::{generate_txn_digest_vec, NewTxnArgs, QuorumCertifiedTxn, TransactionDigest, Txn},
};

macro_rules! rand_in_range {
    ($min:expr, $max:expr) => {{
        use rand::Rng;
        let mut rng = rand::thread_rng();
        rng.gen_range($min..=$max)
    }};
}

pub fn create_mock_full_node_config() -> NodeConfig {
    let data_dir = env::temp_dir();
    let id = Uuid::new_v4().to_string();

    let temp_dir_path = std::env::temp_dir();
    let db_path = temp_dir_path.join(vrrb_core::helpers::generate_random_string());

    let idx = 100;

    let port_start = rand_in_range!(9292, 19292);

    let http_api_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port_start + 1);
    let jsonrpc_server_address =
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port_start + 2);
    let udp_gossip_address =
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port_start + 3);
    let raptorq_gossip_address =
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port_start + 4);
    let rendezvous_local_address =
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port_start + 5);
    let rendezvous_server_address =
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port_start + 6);
    let public_ip_address =
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port_start + 7);
    let grpc_server_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 50051);

    let main_bootstrap_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port_start);
    let bootstrap_node_addresses = vec![main_bootstrap_addr];

    NodeConfigBuilder::default()
        .id(id)
        .idx(idx)
        .data_dir(data_dir)
        .db_path(db_path)
        .node_type(NodeType::Full)
        .bootstrap_node_addresses(bootstrap_node_addresses)
        .bootstrap_config(None)
        .http_api_address(http_api_address)
        .http_api_title(String::from("HTTP Node API"))
        .http_api_version(String::from("1.0"))
        .http_api_shutdown_timeout(Some(Duration::from_secs(5)))
        .raptorq_gossip_address(raptorq_gossip_address)
        .udp_gossip_address(udp_gossip_address)
        .jsonrpc_server_address(jsonrpc_server_address)
        .keypair(Keypair::random())
        .rendezvous_local_address(rendezvous_local_address)
        .rendezvous_server_address(rendezvous_server_address)
        .public_ip_address(public_ip_address)
        .grpc_server_address(grpc_server_address)
        .disable_networking(false)
        .build()
        .unwrap()
}

pub fn create_mock_full_node_config_with_bootstrap(
    bootstrap_node_addresses: Vec<SocketAddr>,
) -> NodeConfig {
    let mut node_config = create_mock_full_node_config();

    node_config.bootstrap_node_addresses = bootstrap_node_addresses;
    node_config
}

pub fn create_mock_bootstrap_node_config() -> NodeConfig {
    let mut node_config = create_mock_full_node_config();

    node_config.bootstrap_node_addresses = vec![];
    node_config.node_type = NodeType::Bootstrap;

    node_config
}

pub fn produce_accounts(n: usize) -> Vec<(Address, Account)> {
    (0..n)
        .map(|_| {
            let kp = generate_account_keypair();
            let mut account = Account::new(kp.1);
            account.credits = 1_000_000_000_000_000_000_000_000_000u128;
            (Address::new(kp.1), account)
        })
        .collect()
}

fn produce_random_claims(n: usize) -> HashSet<Claim> {
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

            Claim::new(kp.miner_kp.1, address, ip_address, signature).unwrap()
        })
        .collect()
}

fn produce_random_txs(accounts: &Vec<(Address, Account)>) -> HashSet<Txn> {
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
                    let pk = validator.clone().0.public_key().to_string();
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
    accounts: Vec<(Address, Account)>,
    n: usize,
    ntx: usize,
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

            let from = Claim::new(kp.miner_kp.1, address, ip_address, signature).unwrap();
            let txs = produce_random_txs(&accounts);
            let claims = produce_random_claims(ntx);

            let txn_list = txs
                .into_iter()
                .map(|txn| {
                    let digest = txn.id();

                    let certified_txn = QuorumCertifiedTxn::new(
                        Vec::new(),
                        Vec::new(),
                        txn,
                        RawSignature::new(),
                        true,
                    );

                    (digest, certified_txn)
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
                keypair.get_miner_secret_key(),
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
    sender: (Address, Account),
    receiver: Address,
    validators: Vec<(String, bool)>,
) -> Txn {
    let (sk, pk) = create_keypair();
    let saddr = sender.0.clone();
    let raddr = receiver;
    let amount = 100u128.pow(2);
    let token = None;

    let validators = validators
        .iter()
        .map(|(k, v)| (k.to_string(), *v))
        .collect();

    let txn_args = NewTxnArgs {
        timestamp: 0,
        sender_address: saddr,
        sender_public_key: pk,
        receiver_address: raddr,
        token,
        amount,
        signature: sk
            .sign_ecdsa(Message::from_hashed_data::<secp256k1::hashes::sha256::Hash>(b"vrrb")),
        validators: Some(validators),
        nonce: sender.1.nonce + 1,
    };

    let mut txn = Txn::new(txn_args);

    txn.sign(&sk);

    let txn_digest_vec = generate_txn_digest_vec(
        txn.timestamp,
        txn.sender_address.to_string(),
        txn.sender_public_key,
        txn.receiver_address.to_string(),
        txn.token.clone(),
        txn.amount,
        txn.nonce,
    );

    let _digest = TransactionDigest::from(txn_digest_vec);

    txn
}
