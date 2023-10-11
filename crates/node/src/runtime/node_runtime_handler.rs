use crate::{node_runtime::NodeRuntime, runtime::NETWORK_TOPIC_STR};
use async_trait::async_trait;
use block::Certificate;
use events::{Event, EventMessage};
use primitives::ConvergencePartialSig;
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
                let assignments = self
                    .handle_node_added_to_peer_list(peer_data.clone())
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                if let Some(assignments) = assignments {
                    let assignments = assignments.into_values().collect();
                    let event = EventMessage::new(
                        Some("network-events".into()),
                        Event::QuorumMembershipAssigmentsCreated(assignments),
                    );
                    self.events_tx
                        .send(event)
                        .await
                        .map_err(|err| TheaterError::Other(err.to_string()))?;
                }
            },
            Event::QuorumMembershipAssigmentCreated(assigned_membership) => {
                self.handle_quorum_membership_assigment_created(assigned_membership.clone())?;
            },
            Event::QuorumMembershipAssigmentsCreated(assignments) => {
                self.handle_quorum_membership_assigments_created(assignments)?
            },
            Event::QuorumElectionStarted(header) => {
                self.handle_quorum_election_started(header)
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },
            Event::MinerElectionStarted(header) => {
                let claims = self.state_driver.read_handle().claim_store_values();

                let results = self
                    .consensus_driver
                    .handle_miner_election_started(header, claims)
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                let winner = results
                    .clone()
                    .into_iter()
                    .next()
                    .ok_or(TheaterError::Other("no winner found".to_string()))?;

                let event = Event::MinerElected(winner);

                let em = EventMessage::new(Some(NETWORK_TOPIC_STR.into()), event);

                self.events_tx
                    .send(em)
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },
            Event::ConvergenceBlockPartialSignatureCreated {
                block_hash,
                public_key_share,
                partial_signature,
            } => {
                // This is likely redundant
            },
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
                self.state_driver.handle_transaction_validated(txn).await?;
            },
            Event::CreateAccountRequested((address, account_bytes)) => {
                // I think we can get rid of this, as we now add accounts
                // when they are a receiver of a transaction
                self.handle_create_account_requested(address.clone(), account_bytes)?;
            },
            Event::AccountUpdateRequested((_address, _account_bytes)) => {
                todo!()
                // This can occur as a result of block application
            },
            Event::UpdateState(block) => {
                if let Err(err) = self.state_driver.update_state(block.hash.clone()) {
                    telemetry::error!("error updating state: {}", err);
                } else {
                    self.events_tx
                        .send(Event::BuildProposalBlock(block).into())
                        .await
                        .map_err(|err| TheaterError::Other(err.to_string()))?;
                }
            },
            Event::BuildProposalBlock(block) => {
                let proposal_block = self
                    .handle_build_proposal_block_requested(block)
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                self.events_tx
                    .send(Event::BroadcastProposalBlock(proposal_block).into())
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },

            Event::ClaimCreated(claim) => {
                // This is likely unneeded
            },
            Event::ClaimReceived(claim) => {
                info!("Storing claim from: {}", claim.address);
                // Claim should be added to pending claims
                // Event to validate claim should be created
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
                self.handle_harvester_signature_received(block_hash, node_id, sig)
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },
            Event::BlockCertificateCreated(certificate) => {
                let confirmed_block = self
                    .handle_block_certificate_created(certificate)
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                self.events_tx
                    .send(Event::UpdateState(confirmed_block).into())
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },
            Event::BlockConfirmed(cert_bytes) => {
                let certificate: Certificate = bincode::deserialize(&cert_bytes)
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                let confirmed_block = self
                    .handle_block_certificate_received(certificate)
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                self.events_tx
                    .send(Event::UpdateState(confirmed_block).into())
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },
            Event::BlockAppended(block_hash) => {
                // This is likely redundant
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
