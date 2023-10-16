mod dag;
mod manager;
mod utils;

pub use dag::*;
pub use manager::*;

#[cfg(test)]
mod tests {
    use std::{
        env,
        net::{IpAddr, Ipv4Addr, SocketAddr},
        sync::{Arc, RwLock},
    };

    use block::{Block, BlockHash};
    use bulldag::{graph::BullDag, vertex::Vertex};
    use integral_db::LeftRightTrie;
    use mempool::LeftRightMempool;
    use miner::test_helpers::{create_address, create_claim};
    use primitives::Address;
    use serial_test::serial;
    use signer::engine::SignerEngine;
    use storage::vrrbdb::types::*;
    use storage::vrrbdb::{RocksDbAdapter, VrrbDb, VrrbDbConfig};
    use storage::{storage_utils::remove_vrrb_data_dir, vrrbdb::types::*};
    use theater::{Actor, ActorImpl, ActorState, Handler};
    use tokio::sync::mpsc::channel;
    use vrrb_core::transactions::TransactionKind;
    use vrrb_core::{account::Account, claim::Claim, keypair::KeyPair};

    use super::*;
    use crate::test_utils::{
        create_blank_certificate, create_keypair, produce_accounts, produce_convergence_block,
        produce_genesis_block, produce_proposal_blocks,
    };

    #[tokio::test]
    #[serial]
    async fn state_runtime_receives_new_txn_event() {
        remove_vrrb_data_dir();
        let _temp_dir_path = env::temp_dir().join("state.json");

        let db_config = VrrbDbConfig::default();

        let db = VrrbDb::new(db_config);
        let mempool = LeftRightMempool::default();

        let dag: Arc<RwLock<BullDag<Block, String>>> = Arc::new(RwLock::new(BullDag::new()));

        let (sk, pk) = create_keypair();
        let addr = create_address(&pk);
        let ip_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
        let signature =
            Claim::signature_for_valid_claim(pk, ip_address, sk.secret_bytes().to_vec()).unwrap();
        let claim = create_claim(&pk, &addr, ip_address, signature);

        let mut state_module = StateManager::new(StateManagerConfig {
            mempool,
            database: db,
            dag: dag.clone(),
            claim,
        });

        state_module
            .handle_new_txn_created(TransactionKind::default())
            .unwrap();
    }

    pub type StateDag = Arc<RwLock<BullDag<Block, BlockHash>>>;

    #[tokio::test]
    #[ignore]
    async fn vrrbdb_should_update_with_new_block() {
        let db_config = VrrbDbConfig::default().with_path(std::env::temp_dir().join("db"));
        let db = VrrbDb::new(db_config);
        let mempool = LeftRightMempool::default();

        let accounts: Vec<(Address, Option<Account>)> = produce_accounts(5);
        let dag: StateDag = Arc::new(RwLock::new(BullDag::new()));

        let keypair = KeyPair::random();
        let mut sig_engine = SignerEngine::new(
            keypair.get_miner_public_key().clone(),
            keypair.get_miner_secret_key().clone(),
        );
        let pk = keypair.get_miner_public_key().clone();
        let addr = create_address(&pk);
        let ip_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
        let signature = Claim::signature_for_valid_claim(
            pk.clone(),
            ip_address,
            keypair.get_miner_secret_key().secret_bytes().to_vec(),
        )
        .unwrap();
        let claim = create_claim(&pk, &addr, ip_address, signature);

        let state_config = StateManagerConfig {
            mempool,
            database: db,
            claim,
            dag: dag.clone(),
        };
        let mut state_module = StateManager::new(state_config);
        let state_res = state_module.extend_accounts(accounts.clone());
        let genesis = produce_genesis_block();

        assert!(state_res.is_ok());

        let gblock: Block = genesis.clone().into();
        let gvtx: Vertex<Block, BlockHash> = gblock.into();
        if let Ok(mut guard) = dag.write() {
            guard.add_vertex(&gvtx);
        }

        let proposals = produce_proposal_blocks(genesis.hash, accounts.clone(), 5, 5, sig_engine);

        let edges: Vec<(Vertex<Block, BlockHash>, Vertex<Block, BlockHash>)> = {
            proposals
                .into_iter()
                .map(|pblock| {
                    let pblock: Block = pblock.into();
                    let pvtx: Vertex<Block, BlockHash> = pblock.into();
                    (gvtx.clone(), pvtx)
                })
                .collect()
        };

        if let Ok(mut guard) = dag.write() {
            edges
                .iter()
                .for_each(|(source, reference)| guard.add_edge((source, reference)));
        }

        let block_hash = produce_convergence_block(dag).unwrap();
        state_module.update_state(block_hash).unwrap();

        state_module.commit();

        let handle = state_module.read_handle();
        let store = handle.state_store_values();

        for (address, _) in accounts.iter() {
            let account = store.get(address).unwrap();
            let digests = account.digests().clone();

            assert_eq!(digests.get_sent().len(), 5);
            assert_eq!(digests.get_recv().len(), 5);
            assert_eq!(digests.get_stake().len(), 0);
        }
    }
}
