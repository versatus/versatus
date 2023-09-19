mod component;
mod dag;
mod handler;
mod manager;
mod types;
mod utils;

pub use component::*;
pub use dag::*;
pub use handler::*;
pub use manager::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use std::{
        env,
        net::{IpAddr, Ipv4Addr, SocketAddr},
        sync::{Arc, RwLock},
    };

    use block::{Block, BlockHash};
    use bulldag::{graph::BullDag, vertex::Vertex};
    use events::{Event, DEFAULT_BUFFER};
    use mempool::LeftRightMempool;
    use miner::test_helpers::{create_address, create_claim};
    use primitives::Address;
    use serial_test::serial;
    use storage::vrrbdb::{VrrbDb, VrrbDbConfig};
    use theater::{Actor, ActorImpl, ActorState};
    use tokio::sync::mpsc::channel;
    use vrrb_core::{account::Account, claim::{Claim, self}, keypair::KeyPair};
    use vrrb_core::transactions::{TransactionKind};

    use crate::test_utils::{create_blank_certificate, _create_dag_module};

    use super::*;
    use crate::test_utils::{
        create_keypair, produce_accounts, produce_convergence_block, produce_genesis_block,
        produce_proposal_blocks,
    };

    #[tokio::test]
    #[serial]
    async fn state_runtime_module_starts_and_stops() {
        let _temp_dir_path = env::temp_dir().join("state.json");

        let (events_tx, _) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);

        let db_config = VrrbDbConfig::default();

        let dag: Arc<RwLock<BullDag<Block, String>>> = Arc::new(RwLock::new(BullDag::new()));

        let db = VrrbDb::new(db_config);
        let mempool = LeftRightMempool::new();

        let (sk, pk) = create_keypair();
        let addr = create_address(&pk);
        let ip_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
        let signature = Claim::signature_for_valid_claim(pk, ip_address, sk.secret_bytes().to_vec()).unwrap();
        let claim = create_claim(&pk, &addr, ip_address, signature);

        let _state_module = StateManager::new(StateManagerConfig {
            events_tx,
            mempool,
            database: db,
            claim: claim,
            dag: dag.clone(),
        });

        let mut state_module = ActorImpl::new(_state_module);

        let (ctrl_tx, mut ctrl_rx) = tokio::sync::broadcast::channel(DEFAULT_BUFFER);

        assert_eq!(state_module.status(), ActorState::Stopped);

        let handle = tokio::spawn(async move {
            state_module.start(&mut ctrl_rx).await.unwrap();
            assert_eq!(state_module.status(), ActorState::Terminating);
        });

        ctrl_tx.send(Event::Stop.into()).unwrap();

        handle.await.unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn state_runtime_receives_new_txn_event() {
        let _temp_dir_path = env::temp_dir().join("state.json");

        let (events_tx, _) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);
        let db_config = VrrbDbConfig::default();

        let db = VrrbDb::new(db_config);
        let mempool = LeftRightMempool::default();

        let dag: Arc<RwLock<BullDag<Block, String>>> = Arc::new(RwLock::new(BullDag::new()));

        let (sk, pk) = create_keypair();
        let addr = create_address(&pk);
        let ip_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
        let signature = Claim::signature_for_valid_claim(pk, ip_address, sk.secret_bytes().to_vec()).unwrap();
        let claim = create_claim(&pk, &addr, ip_address, signature);

        let state_module = StateManager::new(StateManagerConfig {
            events_tx,
            mempool,
            database: db,
            dag: dag.clone(),
            claim,
        });

        let mut state_module = ActorImpl::new(state_module);

        let (ctrl_tx, mut ctrl_rx) = tokio::sync::broadcast::channel(DEFAULT_BUFFER);

        assert_eq!(state_module.status(), ActorState::Stopped);

        let handle = tokio::spawn(async move {
            state_module.start(&mut ctrl_rx).await.unwrap();
        });

        ctrl_tx
            .send(Event::NewTxnCreated(TransactionKind::default()).into())
            .unwrap();

        ctrl_tx.send(Event::Stop.into()).unwrap();

        handle.await.unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn state_runtime_can_publish_events() {
        let _temp_dir_path = env::temp_dir().join("state.json");

        let (events_tx, mut events_rx) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);

        let db_config = VrrbDbConfig::default();

        let db = VrrbDb::new(db_config);
        let mempool = LeftRightMempool::default();

        let dag: StateDag = Arc::new(RwLock::new(BullDag::new()));
        let (sk, pk) = create_keypair();
        let addr = create_address(&pk);
        let ip_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
        let signature = Claim::signature_for_valid_claim(pk, ip_address, sk.secret_bytes().to_vec()).unwrap();
        let claim = create_claim(&pk, &addr, ip_address, signature);

        let state_module = StateManager::new(StateManagerConfig {
            mempool,
            events_tx,
            database: db,
            dag: dag.clone(),
            claim,
        });

        let mut state_module = ActorImpl::new(state_module);

        let events_handle = tokio::spawn(async move {
            let _res = events_rx.recv().await;
        });

        let (ctrl_tx, mut ctrl_rx) = tokio::sync::broadcast::channel(DEFAULT_BUFFER);

        assert_eq!(state_module.status(), ActorState::Stopped);

        let handle = tokio::spawn(async move {
            state_module.start(&mut ctrl_rx).await.unwrap();
        });

        // TODO: implement all state && validation ops

        ctrl_tx
            .send(Event::NewTxnCreated(TransactionKind::default()).into())
            .unwrap();

        ctrl_tx.send(Event::Stop.into()).unwrap();

        handle.await.unwrap();
        events_handle.await.unwrap();
    }

    pub type StateDag = Arc<RwLock<BullDag<Block, BlockHash>>>;

    #[tokio::test]
    #[ignore = "https://github.com/versatus/versatus/issues/471"]
    async fn vrrbdb_should_update_with_new_block() {
        let db_config = VrrbDbConfig::default().with_path(std::env::temp_dir().join("db"));
        let db = VrrbDb::new(db_config);
        let mempool = LeftRightMempool::default();

        let accounts: Vec<(Address, Option<Account>)> = produce_accounts(5);
        let dag: StateDag = Arc::new(RwLock::new(BullDag::new()));
        let (events_tx, _) = channel(100);

        let keypair = KeyPair::random();
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
            events_tx,
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

        let proposals = produce_proposal_blocks(genesis.hash, accounts.clone(), 5, 5);

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

        state_module._commit();

        let handle = state_module._read_handle();
        let store = handle.state_store_values();

        for (address, _) in accounts.iter() {
            let account = store.get(address).unwrap();
            let digests = account.digests().clone();

            assert_eq!(digests.get_sent().len(), 5);
            assert_eq!(digests.get_recv().len(), 5);
            assert_eq!(digests.get_stake().len(), 0);
        }
    }
    #[tokio::test]
    async fn handle_event_block_certificate() {
        let dag_module = _create_dag_module();
        let certificate = create_blank_certificate(dag_module.claim.signature.clone());

        let _message: messr::Message<Event> = Event::BlockCertificateCreated(certificate).into();

        // assert_eq!(
        // ActorState::Running,
        // dag_module.handle(message).await.unwrap()
        // );
    }
}
