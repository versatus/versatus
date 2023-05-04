use std::collections::HashSet;

use async_trait::async_trait;
use events::{Event, EventMessage, EventPublisher};
use lr_trie::ReadHandleFactory;
use patriecia::{db::MemoryDB, inner::InnerTrie};
use primitives::Address;
use storage::vrrbdb::{StateStoreReadHandle, VrrbDb, VrrbDbReadHandle};
use telemetry::info;
use theater::{Actor, ActorId, ActorLabel, ActorState, Handler, TheaterError};
use vrrb_core::{
    account::{Account, UpdateArgs},
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
    pub nonce: u128,
    pub storage: Option<String>,
    pub code: Option<String>,
    pub digest: TransactionDigest,
    pub update_account: UpdateAccount,
}

pub struct StateModuleConfig {
    pub db: VrrbDb,
    pub events_tx: EventPublisher,
}

#[derive(Debug)]
pub struct StateModule {
    db: VrrbDb,
    status: ActorState,
    label: ActorLabel,
    id: ActorId,
    events_tx: EventPublisher,
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

    fn write_txns(&mut self, update_list: HashSet<StateUpdate>) -> Result<()> {
        update_list
            .into_iter()
            .for_each(|update| match &update.update_account {
                UpdateAccount::Sender => {
                    if let Some(args) = self.get_sender_update_args(update) {
                        self.db.update_account(update.address.clone(), args);
                    }
                },
                UpdateAccount::Receiver => {
                    if let Some(args) = self.get_receiver_update_args(update) {
                        self.db.update_account(update.address.clone(), args);
                    }
                },
                UpdateAccount::Claim => {
                    if let Some(args) = self.get_claim_update_args(update) {
                        self.db.update_account(update.address.clone(), args);
                    }
                },
            });

        Ok(())
    }

    fn write_claim(&mut self, claim_list: HashSet<Claim>) -> Result<()> {
        todo!()
    }

    fn insert_account(&mut self, key: Address, account: Account) -> Result<()> {
        self.db
            .insert_account(key, account)
            .map_err(|err| NodeError::Other(err.to_string()))
    }

    fn update_claim(&mut self, update: StateUpdate) {
        todo!()
    }

    fn get_sender_update_args(&mut self, update: StateUpdate) -> Option<UpdateArgs> {
        let handle = self.get_state_store_handle();
        if let Ok(mut account) = handle.get(&update.address) {
            let mut nonce = account.nonce;
            let mut credits = account.credits;
            let debits = account.debits;
            let mut storage = account.storage.clone();
            let mut code = account.code.clone();
            let mut digests = account.digests.clone();

            nonce += 1;
            credits += update.amount;
            storage = update.storage.clone();
            code = update.code.clone();
            digests.insert(nonce, update.digest.clone());

            return Some(UpdateArgs {
                nonce,
                credits: None,
                debits: Some(debits),
                storage: Some(storage.clone()),
                code: Some(code.clone()),
                digests: Some(digests.clone()),
            });
        }

        None
    }

    fn get_receiver_update_args(&mut self, update: StateUpdate) -> Option<UpdateArgs> {
        let handle = self.get_state_store_handle();
        if let Ok(mut account) = handle.get(&update.address) {
            let mut nonce = account.nonce;
            let mut credits = account.credits;
            let debits = account.debits;
            let mut storage = account.storage.clone();
            let mut code = account.code.clone();
            let mut digests = account.digests.clone();

            nonce += 1;
            credits += update.amount;
            storage = update.storage.clone();
            code = update.code.clone();
            digests.insert(nonce, update.digest.clone());

            return Some(UpdateArgs {
                nonce,
                credits: None,
                debits: Some(debits),
                storage: Some(storage.clone()),
                code: Some(code.clone()),
                digests: Some(digests.clone()),
            });
        }

        None
    }

    fn get_claim_update_args(&mut self, update: StateUpdate) -> Option<UpdateArgs> {
        todo!()
    }

    fn get_state_store_handle(&self) -> StateStoreReadHandle {
        self.db.state_store_factory().handle()
    }
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
                if let Ok(account) = decode_from_binary_byte_slice(&account_bytes) {
                    self.update_account(address, account)
                        .map_err(|err| TheaterError::Other(err.to_string()))?;
                }
            },
            Event::NoOp => {},
            _ => {},
        }

        Ok(ActorState::Running)
    }
}

#[cfg(test)]
mod tests {
    use std::env;

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

        let db = VrrbDb::new(db_config);

        let mut state_module = StateModule::new(StateModuleConfig { events_tx, db });

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

        let mut state_module = StateModule::new(StateModuleConfig { events_tx, db });

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

        let mut state_module = StateModule::new(StateModuleConfig { events_tx, db });

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
