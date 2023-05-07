use std::{
    collections::HashSet,
    sync::{Arc, RwLock},
};

use block::{Block, BlockHash, ConvergenceBlock, GenesisBlock, ProposalBlock};
use bulldag::{graph::BullDag, vertex::Vertex};
use node::state_module::{StateModule, StateModuleConfig};
use primitives::Address;
use storage::vrrbdb::{VrrbDb, VrrbDbConfig};
use tokio::sync::mpsc::channel;
use vrrb_core::{account::Account, txn::Txn};

#[tokio::test]
async fn vrrbdb_should_update_with_new_block() {
    // setup necessary components to conduct test,
    // including VrrbDb instance & DAG instance.
    // DAG instance should already have 1 round under its
    // belt, i.e. a GenesisBlock, at least 1 ProposalBlock and
    // at least 1 Convergence block.
    // Provide the StateModule with the necessary configuration
    // including a copy of the VrrbDb and the DAG
    // Provide the BlockHash for the ConvergenceBlock and call the
    // state_module.update_state(); method passing the ConvergenceBlock
    // hash into it.
    // Check the VrrbDb instance to ensure that the transactions and
    // claims in the PropsalBlock(s)/ConvergenceBlock are reflected
    // in the db, including in the StateStore, ClaimStore and
    // TransactionStore
    let (block_hash, mut state_module) = produce_state_module(5, 5);
    let _ = state_module.update_state(block_hash);

    todo!();
}


fn produce_state_module(ntx: usize, npb: usize) -> (BlockHash, StateModule) {
    let (events_tx, _) = channel(100);
    let db_config = VrrbDbConfig::default();
    let mut db = VrrbDb::new(db_config.clone());
    let accounts = populate_db_with_accounts(&mut db, 10);
    let (block_hash, dag) = build_dag(accounts, ntx, npb);
    let dag = Arc::new(RwLock::new(dag.clone()));
    let config = StateModuleConfig {
        db: VrrbDb::new(db_config),
        events_tx,
        dag: dag.clone(),
    };

    (block_hash, StateModule::new(config))
}

fn produce_accounts(n: usize) -> Vec<(Address, Account)> {
    todo!()
}

fn populate_db_with_accounts(db: &mut VrrbDb, n: usize) -> Vec<(Address, Account)> {
    let accounts = produce_accounts(n);
    db.extend_accounts(accounts.clone());

    accounts
}

fn produce_random_txs(n: usize, accounts: &Vec<(Address, Account)>) -> HashSet<Txn> {
    (0..n).into_iter().map(|_| Txn::null_txn()).collect()
}

fn produce_genesis_block() -> GenesisBlock {
    todo!()
}

fn produce_proposal_blocks(
    accounts: Vec<(Address, Account)>,
    n: usize,
    ntx: usize,
) -> Vec<ProposalBlock> {
    let txs = produce_random_txs(ntx, &accounts);
    todo!()
}

fn produce_convergence_block(proposals: Vec<ProposalBlock>) -> ConvergenceBlock {
    todo!()
}

fn build_dag(
    accounts: Vec<(Address, Account)>,
    ntx: usize,
    npb: usize,
) -> (BlockHash, BullDag<Block, BlockHash>) {
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

    let convergence = produce_convergence_block(proposals);
    let c_block: Block = convergence.clone().into();
    let cvtx: Vertex<Block, BlockHash> = c_block.into();

    let c_edges = proposal_vtxs.iter().map(|pvtx| (pvtx, &cvtx)).collect();

    dag.extend_from_edges(c_edges);

    (convergence.hash, dag)
}
