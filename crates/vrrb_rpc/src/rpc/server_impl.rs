use std::{collections::HashMap, str::FromStr};

use async_trait::async_trait;
use block::block::Block;
use block::ClaimHash;
use events::{Event, EventPublisher};
use jsonrpsee::core::Error;
use mempool::MempoolReadHandleFactory;
use primitives::{Address, NodeType, Round};
use secp256k1::{Message, SecretKey};
use sha2::{Digest, Sha256};
use storage::vrrbdb::{Claims, VrrbDbReadHandle};
use telemetry::{debug, error};
use vrrb_config::bootstrap_quorum::QuorumMembershipConfig;
use vrrb_core::claim::Claim;
use vrrb_core::node_health_report::NodeHealthReport;
use vrrb_core::transactions::{RpcTransactionDigest, Transaction, TransactionDigest, TransactionKind};
use vrrb_core::{account::Account, serde_helpers::encode_to_binary};

use super::{
    api::{FullMempoolSnapshot, RpcApiServer},
    SignOpts,
};
use crate::rpc::api::{FullStateSnapshot, RpcTransactionRecord};

#[derive(Debug, Clone)]
pub struct RpcServerImpl {
    pub node_type: NodeType,
    pub vrrbdb_read_handle: VrrbDbReadHandle,
    pub mempool_read_handle_factory: MempoolReadHandleFactory,
    pub events_tx: EventPublisher,
}

#[async_trait]
impl RpcApiServer for RpcServerImpl {
    async fn get_full_state(&self) -> Result<FullStateSnapshot, Error> {
        let values = self.vrrbdb_read_handle.state_store_values();

        Ok(values)
    }

    async fn get_full_mempool(&self) -> Result<FullMempoolSnapshot, Error> {
        let values = self
            .mempool_read_handle_factory
            .values()
            .iter()
            .map(|txn| RpcTransactionRecord::from(txn.clone()))
            .collect();

        Ok(values)
    }

    async fn get_node_type(&self) -> Result<NodeType, Error> {
        Ok(self.node_type)
    }

    //TODO: this should either exist for every transaction type or allow creating multiple types
    async fn create_txn(&self, txn: TransactionKind) -> Result<RpcTransactionRecord, Error> {
        let event = Event::NewTxnCreated(txn.clone());

        debug!("{:?}", event);

        self.events_tx.send(event.into()).await.map_err(|err| {
            error!("could not queue transaction to mempool: {err}");
            Error::Custom(err.to_string())
        })?;

        Ok(RpcTransactionRecord::from(txn))
    }

    async fn get_transaction(
        &self,
        transaction_digest: RpcTransactionDigest,
    ) -> Result<RpcTransactionRecord, Error> {
        // Do we need to check both state AND mempool?
        debug!("Received a getTransaction RPC request");

        let parsed_digest = transaction_digest
            .parse::<TransactionDigest>()
            .map_err(|_err| Error::Custom("unable to parse transaction digest".to_string()))?;

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

    async fn create_account(&self, address: Address, account: Account) -> Result<(), Error> {
        let account_bytes =
            encode_to_binary(&account).map_err(|err| Error::Custom(err.to_string()))?;

        let event = Event::CreateAccountRequested((address.clone(), account_bytes));

        debug!("{:?}", event);

        self.events_tx
            .send(event.clone().into())
            .await
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

        let addr =
            Address::from_str(&account.hash()).map_err(|err| Error::Custom(err.to_string()))?;

        let event = Event::AccountUpdateRequested((addr, account_bytes));

        self.events_tx.send(event.into()).await.map_err(|err| {
            error!("could not update account: {err}");
            Error::Custom(err.to_string())
        })?;

        Ok(())
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

    async fn faucet_drip(&self, address: Address) -> Result<(), Error> {
        todo!()
    }

    async fn sign_transaction(&self, sign_opts: SignOpts) -> Result<String, Error> {
        let payload = format!(
            "{},{},{},{},{},{:?},{}",
            &sign_opts.timestamp,
            &sign_opts.sender_address,
            &sign_opts.sender_public_key,
            &sign_opts.receiver_address,
            &sign_opts.amount,
            &sign_opts.token,
            &sign_opts.nonce
        );

        let mut hasher = Sha256::new();
        hasher.update(payload.as_bytes());
        let payload_hash = hasher.finalize();

        type H = secp256k1::hashes::sha256::Hash;
        let msg = Message::from_hashed_data::<H>(&payload_hash[..]);

        let secret_key_result = SecretKey::from_str(&sign_opts.private_key);

        let secret_key = match secret_key_result {
            Ok(secret_key) => secret_key,
            Err(_) => return Err(Error::Custom("unable to parse secret_key".to_string())),
        };

        Ok(secret_key.sign_ecdsa(msg).to_string())
    }

    async fn get_round(&self) -> Result<Round, Error> {
        todo!()
    }

    async fn get_blocks(&self) -> Result<Vec<Block>, Error> {
        todo!()
    }

    async fn get_program(&self) -> Result<(), Error> {
        todo!()
    }

    async fn call_program(&self) -> Result<(), Error> {
        todo!()
    }

    async fn get_transaction_count(&self, account: Address) -> Result<usize, Error> {
        todo!()
    }

    async fn get_node_health(&self) -> Result<NodeHealthReport, Error> {
        todo!()
    }

    async fn get_claims_by_account_id(&self, address: Address) -> Result<Claims, Error> {
        let claims = self.vrrbdb_read_handle.claim_store_values();
        let claims = claims
            .iter()
            .map(|(_, claim)| claim.clone())
            .filter(|claim| claim.address == address)
            .collect();

        Ok(claims)
    }

    async fn get_claim_hashes(&self) -> Result<Vec<ClaimHash>, Error> {
        let claims = self.vrrbdb_read_handle.claim_store_values();
        let claim_hashes = claims.iter().map(|(_, claim)| claim.hash.clone()).collect();

        Ok(claim_hashes)
    }

    async fn get_claims(&self, claim_hashes: Vec<ClaimHash>) -> Result<Claims, Error> {
        let claims = self.vrrbdb_read_handle.claim_store_values();
        let claims = claims
            .iter()
            .map(|(_, claim)| claim.clone())
            .filter(|claim| claim_hashes.contains(&claim.hash))
            .collect();

        Ok(claims)
    }

    async fn get_membership_config(&self) -> Result<QuorumMembershipConfig, Error> {
        todo!()
    }

    async fn get_last_block(&self) -> Result<Block, Error> {
        todo!()
    }
}
