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
use telemetry::{debug, error};
use tokio::sync::mpsc::UnboundedSender;
use vrrb_core::{
    account::Account,
    event_router::{DirectedEvent, Event, Topic},
    serde_helpers::{encode_to_binary, encode_to_json},
    txn::{NewTxnArgs, TransactionDigest, Txn},
};

use crate::rpc::api::{
    FullMempoolSnapshot,
    FullStateSnapshot,
    RpcApi,
    RpcTransactionDigest,
    RpcTransactionRecord,
};

pub struct RpcServerImpl {
    pub node_type: NodeType,
    pub vrrbdb_read_handle: VrrbDbReadHandle,
    pub mempool_read_handle_factory: MempoolReadHandleFactory,
    pub events_tx: UnboundedSender<DirectedEvent>,
}

#[async_trait]
impl RpcApi for RpcServerImpl {
    async fn get_full_state(&self) -> Result<FullStateSnapshot, Error> {
        let values = self.vrrbdb_read_handle.state_store_values();

        Ok(values)
    }

    async fn get_full_mempool(&self) -> Result<FullMempoolSnapshot, Error> {
        let values = self
            .mempool_read_handle_factory
            .values()
            .iter()
            .map(|txn| {
                let txn = txn.clone();
                let txn = RpcTransactionRecord::from(txn);

                txn
            })
            .collect();

        Ok(values)
    }

    async fn get_node_type(&self) -> Result<NodeType, Error> {
        Ok(self.node_type)
    }

    async fn create_txn(&self, args: NewTxnArgs) -> Result<Txn, Error> {
        let txn = Txn::new(args);
        let event = Event::NewTxnCreated(txn.clone());

        debug!("{:?}", event);

        if self.events_tx.is_closed() {
            let err = Error::Custom("event router is closed".to_string());

            error!("failed to publish write: {:?}", err);

            return Err(err);
        }

        self.events_tx
            .send((Topic::Storage, event))
            .map_err(|err| {
                error!("could not queue transaction to mempool: {err}");
                Error::Custom(err.to_string())
            })?;

        Ok(txn)
    }

    async fn create_account(&self, address: Address, account: Account) -> Result<(), Error> {
        let account_bytes =
            encode_to_binary(&account).map_err(|err| Error::Custom(err.to_string()))?;

        let event = Event::CreateAccountRequested((address.clone(), account_bytes));

        debug!("{:?}", event);

        self.events_tx
            .send((Topic::Storage, event.clone()))
            .map_err(|err| {
                error!("could not create account: {err}");
                Error::Custom(err.to_string())
            })?;

        telemetry::info!("requested account creation for address: {}", address);

        Ok(())
    }

    async fn update_account(&self, account: Account) -> Result<(), Error> {
        debug!("Received an updateAccount RPC request");

        let account_bytes =
            encode_to_binary(&account).map_err(|err| Error::Custom(err.to_string()))?;

        // let event = Event::RequestedAccountUpdate((account.hash, account_bytes));
        // self.events_tx.send((Topic::State, event)).map_err(|err| {
        //     error!("could not update account: {err}");
        //     Error::Custom(err.to_string())
        // })?;

        Ok(())
    }

    async fn get_transaction(
        &self,
        transaction_digest: RpcTransactionDigest,
    ) -> Result<RpcTransactionRecord, Error> {
        // Do we need to check both state AND mempool?
        debug!("Received a getTransaction RPC request");

        let parsed_digest = transaction_digest
            .parse::<TransactionDigest>()
            .map_err(|err| Error::Custom("unable to parse transaction digest".to_string()))?;

        let values = self.vrrbdb_read_handle.transaction_store_values();
        let value = values.get(&parsed_digest);

        match value {
            Some(txn) => {
                let txn_record = RpcTransactionRecord::from(txn.clone());
                Ok(txn_record)
            },
            None => return Err(Error::Custom("unable to find transaction".to_string())),
        }
    }

    async fn list_transactions(
        &self,
        digests: Vec<RpcTransactionDigest>,
    ) -> Result<HashMap<RpcTransactionDigest, RpcTransactionRecord>, Error> {
        debug!("Received a listTransactions RPC request");

        let mut values: HashMap<RpcTransactionDigest, RpcTransactionRecord> = HashMap::new();

        digests.iter().for_each(|digest_string| {
            let parsed_digest = digest_string
                .parse::<TransactionDigest>()
                .unwrap_or_default(); // TODO: report this error

            if let Some(txn) = self
                .vrrbdb_read_handle
                .transaction_store_values()
                .get(&parsed_digest)
            {
                let txn_record = RpcTransactionRecord::from(txn.clone());

                values.insert(txn.digest().to_string(), txn_record);
            }
        });

        Ok(values)
    }

    async fn get_account(&self, address: Address) -> Result<Account, Error> {
        telemetry::info!("retrieving account {address}");

        let values = self.vrrbdb_read_handle.state_store_values();
        let value = values.get(&address);

        debug!("Received getAccount RPC Request: {value:?}");

        match value {
            Some(account) => return Ok(account.to_owned()),
            None => return Err(Error::Custom("unable to find account".to_string())),
        }
    }
}
