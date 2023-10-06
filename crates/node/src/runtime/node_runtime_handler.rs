use crate::node_runtime::NodeRuntime;
use async_trait::async_trait;
use block::Certificate;
use events::{Event, EventMessage, EventPublisher, EventSubscriber, Vote};
use primitives::{
    ConvergencePartialSig, NodeId, NodeType, PublicKey, QuorumId, QuorumType, ValidatorPublicKey,
};
use signer::signer::Signer;
use telemetry::info;
use theater::{ActorId, ActorLabel, ActorState, Handler, TheaterError};

#[async_trait]
impl Handler<EventMessage> for NodeRuntime {
    fn id(&self) -> ActorId {
        self.id.clone()
    }

    fn label(&self) -> ActorLabel {
        format!("NodeRuntime::{}", self.id())
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
            Event::NodeAddedToPeerList(peer_data) => {
                let assigments = self
                    .handle_node_added_to_peer_list(peer_data.clone())
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                if let Some(assigments) = assigments {
                    for (_, assigned_membership) in assigments {
                        let event = Event::QuorumMembershipAssigmentCreated(assigned_membership);
                        let em = EventMessage::new(Some("network-events".into()), event);
                        self.events_tx
                            .send(em)
                            .await
                            .map_err(|err| TheaterError::Other(err.to_string()))?;
                    }
                }
            },
            Event::QuorumMembershipAssigmentCreated(assigned_membership) => {
                let assignments =
                    self.handle_quorum_membership_assigment_created(assigned_membership.clone());
            },
            Event::QuorumElectionStarted(header) => {
                let quorums = self
                    .handle_quorum_election_started(header)
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                let quorum_assignment: Vec<(QuorumType, Vec<(NodeId, PublicKey)>)> = {
                    quorums
                        .clone()
                        .iter()
                        .filter_map(|quorum| {
                            quorum
                                .quorum_type
                                .clone()
                                .map(|qt| (qt.clone(), quorum.members.clone()))
                        })
                        .collect()
                };

                self.consensus_driver
                    .sig_engine
                    .set_quorum_members(quorum_assignment.clone());
                let local_id = self.config.id.clone();
                for (qt, members) in quorum_assignment.iter() {
                    if members
                        .clone()
                        .iter()
                        .any(|(node_id, _)| node_id == &local_id)
                    {
                        self.consensus_driver.quorum_membership =
                            Some(QuorumId::new(qt.clone(), members.clone()));
                        self.consensus_driver.quorum_type = Some(qt.clone());
                    }
                }
            },
            Event::MinerElectionStarted(header) => {
                let claims = self.state_driver.read_handle().claim_store_values();

                let winner = self
                    .consensus_driver
                    .handle_miner_election_started(header, claims)
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                let event = Event::MinerElected(winner);

                let em = EventMessage::new(Some("network-events".into()), event);

                self.events_tx
                    .send(em)
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },
            Event::ConvergenceBlockPartialSignatureCreated {
                block_hash,
                public_key_share,
                partial_signature,
            } => {},
            Event::ConvergenceBlockPrecheckRequested {
                convergence_block,
                block_header,
            } => {
                let resolver = self.mining_driver.clone();
                self.handle_convergence_block_precheck_requested(
                    convergence_block,
                    block_header,
                    resolver,
                )
                .await
                .map_err(|err| TheaterError::Other(err.to_string()))?;
            },
            Event::SignConvergenceBlock(block) => {
                let sig = self
                    .handle_sign_convergence_block(block.clone())
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                let partial_sig = ConvergencePartialSig {
                    sig,
                    block_hash: block.hash,
                };

                self.events_tx
                    .send(Event::ConvergenceBlockPartialSignComplete(partial_sig).into())
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },
            Event::NewTxnCreated(txn) => {
                let txn_hash = self
                    .state_driver
                    .handle_new_txn_created(txn)
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                self.events_tx
                    .send(Event::TxnAddedToMempool(txn_hash.clone()).into())
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },

            Event::TxnValidated(txn) => {
                self.state_driver.handle_transaction_validated(txn);
            },
            Event::CreateAccountRequested((address, account_bytes)) => {
                self.handle_create_account_requested(address.clone(), account_bytes);
            },
            Event::AccountUpdateRequested((_address, _account_bytes)) => {
                todo!()
            },
            Event::UpdateState(block_hash) => {
                if let Err(err) = self.state_driver.update_state(block_hash) {
                    telemetry::error!("error updating state: {}", err);
                }
            },
            Event::ClaimCreated(claim) => {},
            Event::ClaimReceived(claim) => {
                info!("Storing claim from: {}", claim.address);
            },
            Event::BlockReceived(mut block) => {
                let next_event = self
                    .state_driver
                    .handle_block_received(&mut block, self.consensus_driver.sig_engine.clone())
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                self.events_tx
                    .send(next_event.into())
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },
            Event::HarvesterSignatureReceived(block_hash, node_id, sig) => {
                // TODO, refactor into a node_runtime method
                self.handle_harvester_signature_received(
                    block_hash,
                    node_id,
                    sig,
                    self.consensus_driver.sig_engine.clone(),
                )
                .await
                .map_err(|err| TheaterError::Other(err.to_string()))?;
            },
            Event::BlockCertificateCreated(certificate) => {
                self.handle_block_certificate_created(certificate)
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },
            Event::BlockConfirmed(certificate) => {
                let certificate: Certificate = bincode::deserialize(&certificate)
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                self.handle_block_certificate(certificate)
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },
            Event::QuorumFormed => self
                .handle_quorum_formed()
                .await
                .map_err(|err| TheaterError::Other(err.to_string()))?,
            Event::TxnAddedToMempool(txn_hash) => {
                let mempool_reader = self.mempool_read_handle_factory().clone();
                let state_reader = self.state_store_read_handle_factory().clone();
                if let Ok((transaction, validity)) =
                    self.validate_transaction_kind(txn_hash, mempool_reader, state_reader)
                {
                    if let Ok(vote) = self.cast_vote_on_transaction_kind(transaction, validity) {
                        self.events_tx
                            .send(
                                Event::TransactionsValidated {
                                    vote,
                                    quorum_threshold: self.config.threshold_config.threshold
                                        as usize,
                                }
                                .into(),
                            )
                            .await
                            .map_err(|err| TheaterError::Other(err.to_string()))?;
                    }
                }
            },
            Event::TransactionsValidated {
                vote,
                quorum_threshold,
            } => {
                self.events_tx
                    .send(Event::BroadcastTransactionVote(vote).into())
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },
            Event::NoOp => {},
            _ => {},
        }

        Ok(ActorState::Running)
    }
}
