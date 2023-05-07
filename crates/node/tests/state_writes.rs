use std::{
    collections::HashSet,
    net::SocketAddr,
    sync::{Arc, RwLock},
};

use block::{Block, BlockHash, GenesisBlock, ProposalBlock};
use bulldag::{graph::BullDag, vertex::Vertex};
use hbbft::crypto::SecretKeyShare;
use miner::test_helpers::*;
use node::state_module::{StateModule, StateModuleConfig};
use primitives::{generate_account_keypair, Address};
use secp256k1::Message;
use storage::vrrbdb::{VrrbDb, VrrbDbConfig};
use tokio::sync::mpsc::channel;
use vrrb_core::{
    account::Account,
    claim::Claim,
    keypair::Keypair,
    txn::{generate_txn_digest_vec, NewTxnArgs, TransactionDigest, Txn},
};

#[tokio::test]
#[ignore]
async fn vrrbdb_should_update_with_new_block() {
    let (accounts, block_hash, mut state_module) = produce_state_module(5, 5);
    let _ = state_module.update_state(block_hash);

    let handle = state_module.read_handle();
    let account_values = handle.state_store_values();
    accounts.iter().for_each(|(address, _)| {
        if let Some(account) = account_values.get(address) {
            let digests = account.digests;
            assert!(digests.len() > 0);
            assert!(digests.get_sent().len() > 0);
            assert!(digests.get_recv().len() > 0);
        }
    });

    todo!();
}


fn produce_state_module(
    ntx: usize,
    npb: usize,
) -> (Vec<(Address, Account)>, BlockHash, StateModule) {
    let (events_tx, _) = channel(100);
    let db_config = VrrbDbConfig::default();
    let mut db = VrrbDb::new(db_config.clone());
    let accounts = populate_db_with_accounts(&mut db, 10);
    let (block_hash, dag) = build_dag(accounts.clone(), ntx, npb);
    let dag = dag.clone();
    let config = StateModuleConfig {
        db: VrrbDb::new(db_config),
        events_tx,
        dag: dag.clone(),
    };

    (accounts, block_hash, StateModule::new(config))
}

fn produce_accounts(n: usize) -> Vec<(Address, Account)> {
    (0..n)
        .into_iter()
        .map(|_| {
            let kp = generate_account_keypair();
            let mut account = Account::new(kp.1.clone());
            account.credits = 1_000_000_000_000_000_000_000_000_000u128;
            (Address::new(kp.1.clone()), Account::new(kp.1.clone()))
        })
        .collect()
}

fn populate_db_with_accounts(db: &mut VrrbDb, n: usize) -> Vec<(Address, Account)> {
    let accounts = produce_accounts(n);
    db.extend_accounts(accounts.clone());
    accounts
}

fn produce_random_claims(n: usize) -> HashSet<Claim> {
    create_claims(n).into_iter().collect()
}

fn produce_random_txs(accounts: &Vec<(Address, Account)>) -> HashSet<Txn> {
    accounts
        .clone()
        .iter()
        .enumerate()
        .map(|(idx, (address, account))| {
            let sender = accounts[idx];
            let receiver: (Address, Account);
            if (idx + 1) == accounts.len() {
                let receiver = accounts[0];
            } else {
                let receiver = accounts[idx + 1];
            }
            create_txn_from_accounts(sender, receiver.0)
        })
        .collect()
}

fn produce_genesis_block() -> GenesisBlock {
    mine_genesis().unwrap()
}


fn produce_proposal_blocks(
    last_block_hash: BlockHash,
    accounts: Vec<(Address, Account)>,
    n: usize,
    ntx: usize,
    last_block_hash: BlockHash,
) -> Vec<ProposalBlock> {
    (0..n)
        .into_iter()
        .map(|_| {
            let kp = Keypair::random();
            let address = Address::new(kp.miner_kp.1.clone());
            let ip_address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
            let signature = Claim::signature_for_valid_claim(
                kp.miner_kp.1.clone(),
                ip_address.clone(),
                kp.get_miner_secret_key().secret_bytes().to_vec(),
            )
            .unwrap();
            let from = Claim::new(
                kp.miner_kp.1.clone(),
                address.clone(),
                ip_address,
                signature,
            )
            .unwrap();
            let txs = produce_random_txs(&accounts);
            let claims = produce_random_claims(ntx);
            let txn_list = txs.into_iter().map(|tx| (tx.into(), tx)).collect();
            let claim_list = claims
                .into_iter()
                .map(|claim| (claim.hash, claim))
                .collect();
            ProposalBlock::build(
                last_block_hash,
                0,
                0,
                txn_list,
                claim_list,
                from,
                SecretKeyShare::default(),
            )
        })
        .collect()
}

fn produce_convergence_block(dag: &mut Arc<RwLock<BullDag<Block, BlockHash>>>) -> BlockHash {
    mine_next_convergence_block(dag)
}

fn build_dag(
    accounts: Vec<(Address, Account)>,
    ntx: usize,
    npb: usize,
) -> (BlockHash, Arc<RwLock<BullDag<Block, BlockHash>>>) {
    let mut dag = BullDag::new();

    let genesis = produce_genesis_block();
    let block: Block = genesis.clone().into();
    let genesis_vtx: Vertex<Block, BlockHash> = block.into();
    dag.add_vertex(&genesis_vtx);
    let proposals = produce_proposal_blocks(accounts, npb, ntx, genesis.hash.clone());
    let proposal_vtxs: Vec<Vertex<Block, BlockHash>> = {
        proposals
            .iter()
            .map(|pblock| {
                let block: Block = pblock.clone().into();
                let vtx: Vertex<Block, BlockHash> = block.into();
                vtx
            })
            .collect()
    };

    let edges = proposal_vtxs
        .iter()
        .map(|pvtx| (&genesis_vtx, pvtx))
        .collect();

    dag.extend_from_edges(edges);

    let mut dag = Arc::new(RwLock::new(dag));

    let convergence = produce_convergence_block(&mut dag.clone());

    (convergence, dag)
}

pub(crate) fn create_txn_from_accounts(sender: (Address, Account), receiver: Address) -> Txn {
    let (sk, pk) = create_keypair();
    let (rsk, rpk) = create_keypair();
    let saddr = sender.0.clone();
    let raddr = receiver.clone();
    let amount = 100u128.pow(2);
    let token = None;

    let txn_args = NewTxnArgs {
        timestamp: 0,
        sender_address: saddr,
        sender_public_key: pk.clone(),
        receiver_address: raddr,
        token,
        amount,
        signature: sk
            .sign_ecdsa(Message::from_hashed_data::<secp256k1::hashes::sha256::Hash>(b"vrrb")),
        validators: None,
        nonce: sender.1.nonce + 1,
    };

    let mut txn = Txn::new(txn_args);

    txn.sign(&sk);

    let txn_digest_vec = generate_txn_digest_vec(
        txn.timestamp,
        txn.sender_address.to_string(),
        txn.sender_public_key.clone(),
        txn.receiver_address.to_string(),
        txn.token.clone(),
        txn.amount,
        txn.nonce,
    );

    let digest = TransactionDigest::from(txn_digest_vec);

    txn
}
