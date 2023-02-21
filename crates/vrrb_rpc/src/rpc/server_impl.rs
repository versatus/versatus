use std::{collections::HashMap, net::SocketAddr};

use async_trait::async_trait;
use jsonrpsee::{
    core::Error,
    server::{ServerBuilder, SubscriptionSink},
    types::SubscriptionResult,
};
use mempool::MempoolReadHandleFactory;
use primitives::{Address, NodeType};
use storage::vrrbdb::VrrbDbReadHandle;
use tokio::sync::mpsc::UnboundedSender;
use vrrb_core::{
    account::Account,
    event_router::{DirectedEvent, Event, Topic},
    txn::{NewTxnArgs, TransactionDigest, Txn},
};

use super::api::{CreateTxnArgs, FullMempoolSnapshot};
use crate::rpc::api::{FullStateSnapshot, RpcServer};

pub struct RpcServerImpl {
    pub node_type: NodeType,
    pub vrrbdb_read_handle: VrrbDbReadHandle,
    pub mempool_read_handle_factory: MempoolReadHandleFactory,
    pub events_tx: UnboundedSender<DirectedEvent>,
}

#[async_trait]
impl RpcServer for RpcServerImpl {
    async fn get_full_state(&self) -> Result<FullStateSnapshot, Error> {
        let values = self.vrrbdb_read_handle.state_store_values();

        Ok(values)
    }

    async fn get_full_mempool(&self) -> Result<FullMempoolSnapshot, Error> {
        let values = self.mempool_read_handle_factory.values();

        Ok(values)
    }

    async fn get_node_type(&self) -> Result<NodeType, Error> {
        Ok(self.node_type)
    }

    async fn create_txn(&self, args: Txn) -> Result<(), Error> {
        let event = Event::NewTxnCreated(args);

        #[cfg(debug_assertions)]
        println!("{:?}", event);
        self.events_tx
            .send((Topic::Transactions, event))
            .map_err(|err| {
                telemetry::error!("could not queue transaction to mempool: {err}");
                Error::Custom(err.to_string())
            })?;

        Ok(())
    }

    async fn create_account(&self, address: Address, account: Account) -> Result<(), Error> {
        let account_bytes =
            serde_json::to_vec(&account).map_err(|err| Error::Custom(err.to_string()))?;
        let event = Event::AccountCreated((address, account_bytes));

        #[cfg(debug_assertions)]
        println!("{:?}", event.clone());

        self.events_tx.send((Topic::State, event)).map_err(|err| {
            telemetry::error!("could not create account: {err}");
            Error::Custom(err.to_string())
        })?;

        Ok(())
    }

    async fn update_account(&self, account: Account) -> Result<(), Error> {
        #[cfg(debug_assertions)]
        println!("Received an updateAccount RPC request");

        let account_bytes =
            serde_json::to_vec(&account).map_err(|err| Error::Custom(err.to_string()))?;

        let event = Event::UpdateAccount(account_bytes);

        self.events_tx.send((Topic::State, event)).map_err(|err| {
            telemetry::error!("could not update account: {err}");
            Error::Custom(err.to_string())
        })?;

        Ok(())
    }

    async fn get_transaction(&self, transaction_digest: TransactionDigest) -> Result<Txn, Error> {
        // Do we need to check both state AND mempool?
        #[cfg(debug_assertions)]
        println!("Received a getTransaction RPC request");
        let values = self.vrrbdb_read_handle.transaction_store_values();
        let value = values.get(&transaction_digest);

        match value {
            Some(txn) => return Ok(txn.to_owned()),
            None => return Err(Error::Custom("unable to find transaction".to_string())),
        }
    }

    async fn list_transactions(
        &self,
        digests: Vec<TransactionDigest>,
    ) -> Result<HashMap<TransactionDigest, Txn>, Error> {
        #[cfg(debug_assertions)]
        println!("Received a listTransactions RPC request");
        let mut values: HashMap<TransactionDigest, Txn> = HashMap::new();
        digests.iter().for_each(|digest| {
            if let Some(txn) = self
                .vrrbdb_read_handle
                .transaction_store_values()
                .get(digest)
            {
                values.insert(digest.clone(), txn.clone());
            }
        });

        Ok(values)
    }

    async fn get_account(&self, address: Address) -> Result<Account, Error> {
        let values = self.vrrbdb_read_handle.state_store_values();
        let value = values.get(&address);

        #[cfg(debug_assertions)]
        println!("Received getAccount RPC Request");

        match value {
            Some(account) => return Ok(account.to_owned()),
            None => return Err(Error::Custom("unable to find account".to_string())),
        }
    }
}
