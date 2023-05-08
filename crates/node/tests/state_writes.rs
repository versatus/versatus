use std::{
    collections::HashSet,
    sync::{Arc, RwLock},
};

use block::{Block, BlockHash, ConvergenceBlock, GenesisBlock, ProposalBlock};
use bulldag::{graph::BullDag, vertex::Vertex};
use miner::test_helpers::{
    build_single_proposal_block,
    create_claim,
    create_claims,
    create_keypair,
    mine_genesis,
    mine_next_convergence_block,
};
use node::state_module::{StateModule, StateModuleConfig};
use primitives::{generate_account_keypair, Address};
use secp256k1::Message;
use storage::vrrbdb::{VrrbDb, VrrbDbConfig};
use tokio::sync::mpsc::channel;
use vrrb_core::{
    account::Account,
    claim::Claim,
    txn::{generate_txn_digest_vec, NewTxnArgs, TransactionDigest, Txn},
};

#[tokio::test]
async fn vrrbdb_should_update_with_new_block() {
    let (block_hash, mut state_module) = produce_state_module(5, 5);
    let _ = state_module.update_state(block_hash);

    // Check that transactions in the ProposalBlocks are reflected
    // in the StateModule

    todo!();
}


fn produce_state_module(ntx: usize, npb: usize) -> (BlockHash, StateModule) {
    let (events_tx, _) = channel(100);
    let db_config = VrrbDbConfig::default();
    let mut db = VrrbDb::new(db_config.clone());
    let accounts = populate_db_with_accounts(&mut db, 10);
    let (block_hash, dag) = build_dag(accounts, ntx, npb);
    let dag = dag.clone();
    let config = StateModuleConfig {
        db: VrrbDb::new(db_config),
        events_tx,
        dag: dag.clone(),
    };

    (block_hash, StateModule::new(config))
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
    accounts: Vec<(Address, Account)>,
    n: usize,
    ntx: usize,
) -> Vec<ProposalBlock> {
    let proposals: Vec<ProposalBlock> = (0..n)
        .into_iter()
        .map(|_| {
            let txs = produce_random_txs(&accounts);
            let claims = produce_random_claims(ntx);
            todo!()
        })
        .collect();
    todo!()
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
    let block: Block = genesis.into();
    let genesis_vtx: Vertex<Block, BlockHash> = block.into();
    dag.add_vertex(&genesis_vtx);
    let proposals = produce_proposal_blocks(accounts, npb, ntx);
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
    let amount = 10000u128.pow(2);
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
