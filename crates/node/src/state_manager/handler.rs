use async_trait::async_trait;
use events::{Event, EventMessage};
use telemetry::info;
use theater::{ActorId, ActorLabel, ActorState, Handler, TheaterError};
use vrrb_core::serde_helpers::decode_from_binary_byte_slice;
use vrrb_core::transactions::Transaction;

use crate::state_manager::StateManager;

#[async_trait]
impl Handler<EventMessage> for StateManager {
    fn id(&self) -> ActorId {
        self.id.clone()
    }

    fn label(&self) -> ActorLabel {
        format!("StateManager::{}", self.id())
    }

    fn status(&self) -> ActorState {
        self.status.clone()
    }

    fn set_status(&mut self, actor_status: ActorState) {
        self.status = actor_status;
    }

    fn on_start(&self) {
        info!("{} starting", self.label());
    }

    fn on_stop(&self) {
        info!("{} received stop signal. Stopping", self.label());
    }

    async fn handle(&mut self, event: EventMessage) -> theater::Result<ActorState> {
        match event.into() {
            Event::Stop => {
                return Ok(ActorState::Stopped);
            },

            Event::NewTxnCreated(txn) => {
                info!("Storing transaction in mempool for validation");

                let txn_hash = txn.id();

                let _mempool_size = self
                    .mempool
                    .insert(txn)
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                self.events_tx
                    .send(Event::TxnAddedToMempool(txn_hash.clone()).into())
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                info!("Transaction {} sent to mempool", txn_hash);

                // if self.mempool.size_in_kilobytes() >= MEMPOOL_THRESHOLD_SIZE
                //     && self.cutoff_transaction.is_none()
                // {
                //     info!("mempool threshold reached");
                //     self.cutoff_transaction = Some(txn_hash.clone());
                //
                //     let event = Event::MempoolSizeThesholdReached {
                //         cutoff_transaction: txn_hash,
                //     };
                //
                //     self.events_tx
                //         .send(event.into())
                //         .await
                //         .map_err(|err|
                // TheaterError::Other(err.to_string()))?; }
            },

            Event::TxnValidated(txn) => {
                self.mempool
                    .remove(&txn.id())
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                self.confirm_txn(txn)
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },

            Event::CreateAccountRequested((address, account_bytes)) => {
                info!(
                    "creating account {address} with new state",
                    address = address.to_string()
                );

                if let Ok(account) = decode_from_binary_byte_slice(&account_bytes) {
                    self.insert_account(address.clone(), account)
                        .map_err(|err| TheaterError::Other(err.to_string()))?;

                    info!("account {address} created", address = address.to_string());
                }
            },
            Event::AccountUpdateRequested((_address, _account_bytes)) => {
                //                if let Ok(account) =
                // decode_from_binary_byte_slice(&account_bytes) {
                // self.update_account(address, account)
                // .map_err(|err| TheaterError::Other(err.to_string()))?;
                //               }
                todo!()
            },
            Event::UpdateState(block) => {
                
                //if let Err(err) = self.update_state(block.hash) {
                //    telemetry::error!("error updating state: {}", err);
                //}
            },
            Event::ClaimCreated(_claim) => {},
            Event::ClaimReceived(claim) => {
                info!("Storing claim from: {}", claim.address);
            },
            Event::BlockReceived(block) => {
                self.handle_block_received(&mut block)
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },
            Event::BlockCertificateCreated(certificate) => {
                self.block_certificate_created(certificate)
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },
            Event::HarvesterPublicKeyReceived(public_key_set) => {
                self.dag.set_harvester_pubkeys(public_key_set)
            },

            Event::TransactionCertificateCreated { txn, .. } => {
                // TODO: forward arguments
                let _ = self.handle_transaction_certificate_created(txn);
            },

            Event::NoOp => {},
            _ => {},
        }

        Ok(ActorState::Running)
    }
}
