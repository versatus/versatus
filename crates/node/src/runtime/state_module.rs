#![allow(unused)]
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    iter::FromIterator,
    sync::{Arc, RwLock},
};

use async_trait::async_trait;
use block::Block;
use bulldag::graph::BullDag;
use events::{Event, EventMessage, EventPublisher};
use lr_trie::ReadHandleFactory;
use patriecia::{db::MemoryDB, inner::InnerTrie};
use primitives::Address;
use storage::vrrbdb::{StateStoreReadHandle, VrrbDb, VrrbDbReadHandle};
use telemetry::info;
use theater::{Actor, ActorId, ActorLabel, ActorState, Handler, TheaterError};
use vrrb_core::{
    account::{Account, AccountNonce, UpdateArgs},
    claim::Claim,
    serde_helpers::decode_from_binary_byte_slice,
    txn::{Token, TransactionDigest, Txn},
};

use crate::{result::Result, NodeError};

#[derive(Debug)]
pub enum UpdateAccount {
    Sender,
    Receiver,
    Claim,
}

#[derive(Debug)]
pub struct StateUpdate {
    pub address: Address,
    pub token: Option<Token>,
    pub amount: u128,
    pub nonce: u32,
    pub storage: Option<String>,
    pub code: Option<String>,
    pub digest: TransactionDigest,
    pub update_account: UpdateAccount,
}

impl Into<UpdateArgs> for StateUpdate {
    fn into(self) -> UpdateArgs {
        let mut digests: HashMap<AccountNonce, TransactionDigest> = HashMap::new();
        digests.insert(self.nonce, self.digest);
        match &self.update_account {
            UpdateAccount::Sender => UpdateArgs {
                address: self.address,
                nonce: self.nonce,
                credits: None,
                debits: Some(self.amount),
                storage: Some(self.storage.clone()),
                code: Some(self.code.clone()),
                digests: Some(digests.clone()),
            },
            UpdateAccount::Receiver => UpdateArgs {
                address: self.address,
                nonce: self.nonce,
                credits: Some(self.amount),
                debits: None,
                storage: Some(self.storage.clone()),
                code: Some(self.code.clone()),
                digests: Some(digests.clone()),
            },
            UpdateAccount::Claim => UpdateArgs {
                address: self.address,
                nonce: self.nonce,
                credits: None,
                debits: None,
                storage: None,
                code: None,
                digests: None,
            },
        }
    }
}

pub struct StateModuleConfig {
    pub db: VrrbDb,
    pub events_tx: EventPublisher,
    pub dag: Arc<RwLock<BullDag<Block, String>>>,
}

#[derive(Debug)]
pub struct StateModule {
    db: VrrbDb,
    status: ActorState,
    label: ActorLabel,
    id: ActorId,
    events_tx: EventPublisher,
    dag: Arc<RwLock<BullDag<Block, String>>>,
}

/// StateModule manages all state persistence and updates within VrrbNodes
/// it runs as an indepdendant module such that it can be enabled and disabled
/// as necessary.
impl StateModule {
    pub fn new(config: StateModuleConfig) -> Self {
        Self {
            db: config.db,
            events_tx: config.events_tx,
            status: ActorState::Stopped,
            label: String::from("State"),
            id: uuid::Uuid::new_v4().to_string(),
            dag: config.dag.clone(),
        }
    }
}

impl StateModule {
    fn name(&self) -> String {
        String::from("State")
    }

    /// Produces a reader factory that can be used to generate read handles into
    /// the state tree.
    #[deprecated(note = "use self.read_handle instead")]
    pub fn factory(&self) -> ReadHandleFactory<InnerTrie<MemoryDB>> {
        // TODO: make this method return a custom factory
        todo!()
    }

    pub fn read_handle(&self) -> VrrbDbReadHandle {
        self.db.read_handle()
    }

    async fn confirm_txn(&mut self, txn: Txn) -> Result<()> {
        let txn_hash = txn.id();

        info!("Storing transaction {txn_hash} in confirmed transaction store");

        //TODO: call checked methods instead
        self.db.insert_transaction(txn)?;

        let event = Event::TxnAddedToMempool(txn_hash);

        self.events_tx.send(event.into()).await?;

        Ok(())
    }

    fn update_state(&mut self, update_list: HashSet<StateUpdate>) -> Result<()> {
        consolidate_update_args(get_update_args(update_list))
            .into_iter()
            .for_each(|(_, args)| {
                let _ = self.db.update_account(args);
            });

        Ok(())
    }

    fn insert_account(&mut self, key: Address, account: Account) -> Result<()> {
        self.db
            .insert_account(key, account)
            .map_err(|err| NodeError::Other(err.to_string()))
    }

    fn get_state_store_handle(&self) -> StateStoreReadHandle {
        self.db.state_store_factory().handle()
    }
}

fn get_update_args(updates: HashSet<StateUpdate>) -> HashSet<UpdateArgs> {
    updates.into_iter().map(|update| update.into()).collect()
}

fn consolidate_update_args(updates: HashSet<UpdateArgs>) -> HashMap<Address, UpdateArgs> {
    let mut consolidated_updates: HashMap<Address, UpdateArgs> = HashMap::new();

    for update in updates.into_iter() {
        let address = update.address.clone();

        consolidated_updates
            .entry(address)
            .and_modify(|existing_update| {
                existing_update.nonce = existing_update.nonce.max(update.nonce);
                existing_update.credits = match (existing_update.credits, update.credits) {
                    (Some(a), Some(b)) => Some(a + b),
                    (a, None) => a,
                    (_, b) => b,
                };
                existing_update.debits = match (existing_update.debits, update.debits) {
                    (Some(a), Some(b)) => Some(a + b),
                    (a, None) => a,
                    (_, b) => b,
                };
                existing_update.storage = update.storage.clone(); // TODO: Update this to use the most recent value
                existing_update.code = update.code.clone(); // TODO: Update this to use the most recent value
                if let Some(digests) = update.digests.clone() {
                    existing_update
                        .digests
                        .get_or_insert(HashMap::new())
                        .extend(digests);
                }
            })
            .or_insert(update);
    }

    consolidated_updates
}


#[async_trait]
impl Handler<EventMessage> for StateModule {
    fn id(&self) -> ActorId {
        self.id.clone()
    }

    fn label(&self) -> ActorLabel {
        self.name()
    }

    fn status(&self) -> ActorState {
        self.status.clone()
    }

    fn set_status(&mut self, actor_status: ActorState) {
        self.status = actor_status;
    }

    fn on_start(&self) {
        info!("{}-{} starting", self.label(), self.id(),);
    }

    fn on_stop(&self) {
        info!(
            "{}-{} received stop signal. Stopping",
            self.name(),
            self.label()
        );
    }

    async fn handle(&mut self, event: EventMessage) -> theater::Result<ActorState> {
        match event.into() {
            Event::Stop => {
                return Ok(ActorState::Stopped);
            },

            Event::TxnValidated(txn) => {
                self.confirm_txn(txn)
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },

            Event::CreateAccountRequested((address, account_bytes)) => {
                telemetry::info!(
                    "creating account {address} with new state",
                    address = address.to_string()
                );

                if let Ok(account) = decode_from_binary_byte_slice(&account_bytes) {
                    self.insert_account(address.clone(), account)
                        .map_err(|err| TheaterError::Other(err.to_string()))?;

                    telemetry::info!("account {address} created", address = address.to_string());
                }
            },
            Event::AccountUpdateRequested((address, account_bytes)) => {
                //                if let Ok(account) =
                // decode_from_binary_byte_slice(&account_bytes) {
                // self.update_account(address, account)
                // .map_err(|err| TheaterError::Other(err.to_string()))?;
                //               }
                todo!()
            },
            Event::NoOp => {},
            _ => {},
        }

        Ok(ActorState::Running)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env,
        sync::{Arc, RwLock},
    };

    use block::Block;
    use bulldag::graph::BullDag;
    use events::{Event, DEFAULT_BUFFER};
    use serial_test::serial;
    use storage::vrrbdb::VrrbDbConfig;
    use theater::ActorImpl;
    use vrrb_core::txn::null_txn;

    use super::*;

    #[tokio::test]
    #[serial]
    async fn state_runtime_module_starts_and_stops() {
        let temp_dir_path = env::temp_dir().join("state.json");

        let (events_tx, _) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);

        let db_config = VrrbDbConfig::default();

        let dag: Arc<RwLock<BullDag<Block, String>>> = Arc::new(RwLock::new(BullDag::new()));

        let db = VrrbDb::new(db_config);

        let mut state_module = StateModule::new(StateModuleConfig {
            events_tx,
            db,
            dag: dag.clone(),
        });

        let mut state_module = ActorImpl::new(state_module);

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
        let temp_dir_path = env::temp_dir().join("state.json");

        let (events_tx, _) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);
        let db_config = VrrbDbConfig::default();

        let db = VrrbDb::new(db_config);

        let dag: Arc<RwLock<BullDag<Block, String>>> = Arc::new(RwLock::new(BullDag::new()));

        let mut state_module = StateModule::new(StateModuleConfig {
            events_tx,
            db,
            dag: dag.clone(),
        });

        let mut state_module = ActorImpl::new(state_module);

        let (ctrl_tx, mut ctrl_rx) = tokio::sync::broadcast::channel(DEFAULT_BUFFER);

        assert_eq!(state_module.status(), ActorState::Stopped);

        let handle = tokio::spawn(async move {
            state_module.start(&mut ctrl_rx).await.unwrap();
        });

        ctrl_tx
            .send(Event::NewTxnCreated(null_txn()).into())
            .unwrap();

        ctrl_tx.send(Event::Stop.into()).unwrap();

        handle.await.unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn state_runtime_can_publish_events() {
        let temp_dir_path = env::temp_dir().join("state.json");

        let (events_tx, mut events_rx) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);

        let db_config = VrrbDbConfig::default();

        let db = VrrbDb::new(db_config);

        let dag: Arc<RwLock<BullDag<Block, String>>> = Arc::new(RwLock::new(BullDag::new()));

        let mut state_module = StateModule::new(StateModuleConfig {
            events_tx,
            db,
            dag: dag.clone(),
        });

        let mut state_module = ActorImpl::new(state_module);

        let events_handle = tokio::spawn(async move {
            let res = events_rx.recv().await;
        });

        let (ctrl_tx, mut ctrl_rx) = tokio::sync::broadcast::channel(DEFAULT_BUFFER);

        assert_eq!(state_module.status(), ActorState::Stopped);

        let handle = tokio::spawn(async move {
            state_module.start(&mut ctrl_rx).await.unwrap();
        });

        // TODO: implement all state && validation ops

        ctrl_tx
            .send(Event::NewTxnCreated(null_txn()).into())
            .unwrap();

        ctrl_tx.send(Event::Stop.into()).unwrap();

        handle.await.unwrap();
        events_handle.await.unwrap();
    }
}
