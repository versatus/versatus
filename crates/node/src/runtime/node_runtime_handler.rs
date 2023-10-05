use async_trait::async_trait;
use block::Certificate;
use dkg_engine::dkg::DkgGenerator;
use events::{Event, EventMessage, EventPublisher, EventSubscriber, Vote};
use primitives::{NodeId, NodeType, ValidatorPublicKey};
use signer::signer::Signer;
use telemetry::info;
use theater::{Actor, ActorId, ActorImpl, ActorLabel, ActorState, Handler, TheaterError};
use vrrb_config::{QuorumMember, QuorumMembershipConfig};
use vrrb_core::serde_helpers::decode_from_binary_byte_slice;

use crate::{consensus::ConsensusModule, node_runtime::NodeRuntime, state_reader::StateReader};

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

                let (part, node_id) =
                    self.generate_partial_commitment_message().map_err(|err| {
                        telemetry::error!("{}", err);
                        TheaterError::Other(err.to_string())
                    })?;

                let event = Event::PartCommitmentCreated(node_id, part);

                let em = EventMessage::new(Some("network-events".into()), event);

                self.events_tx
                    .send(em)
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },

            Event::PartCommitmentCreated(node_id, part) => {
                let (receiver_id, sender_id, ack) = self
                    .handle_part_commitment_created(node_id.clone(), part)
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                let event = Event::PartCommitmentAcknowledged {
                    node_id,
                    sender_id: self.config.id.clone(),
                    ack,
                };

                let em = EventMessage::new(Some("network-events".into()), event);

                self.events_tx
                    .send(em)
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },

            Event::PartCommitmentAcknowledged {
                node_id,
                sender_id,
                ack,
            } => {
                self.consensus_driver
                    .handle_part_commitment_acknowledged(node_id, sender_id, ack)?;
            },

            Event::QuorumElectionStarted(header) => {
                self.handle_quorum_election_started(header).map_err(|err| {
                    TheaterError::Other(err.to_string())
                })?;
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

            Event::TransactionCertificateRequested {
                votes,
                txn_id,
                quorum_key,
                farmer_id,
                txn,
                quorum_threshold,
            } => {
                self.consensus_driver
                    .handle_transaction_certificate_requested(
                        votes,
                        txn_id,
                        quorum_key,
                        farmer_id,
                        txn,
                        quorum_threshold,
                    );
            },

            // This certifies txns once vote threshold is reached.
            Event::TransactionCertificateCreated {
                votes,
                signature,
                digest,
                /// OUtput of the program executed
                execution_result,
                farmer_id,
                txn,
                is_valid,
            } => {
                // TODO: refactor process
            },

            // Mines proposal block after every X seconds.
            Event::ProposalBlockMineRequestCreated {
                ref_hash,
                round,
                epoch,
                claim,
            } => {
                self.handle_proposal_block_mine_request_created(ref_hash, round, epoch, claim);
            },
            // it sends a job to sign the convergence block using the signature
            // provider
            Event::ConvergenceBlockSignatureRequested(block) => {
                //     TODO: merge with ConvergenceBlockPartialSignatureCreated so it all happens
                //     within one function call
                //     if let Some(sig_provider) = self.sig_provider.clone() {
                //         let _ = self
                //             .sync_jobs_sender
                //             .send(Job::SignConvergenceBlock(sig_provider, block));
                //     }
            },

            // Process the job result of signing convergence block and adds the
            // partial signature to the cache for certificate generation
            Event::ConvergenceBlockPartialSignatureCreated {
                block_hash,
                public_key_share,
                partial_signature,
            } => {
                self.consensus_driver
                    .handle_convergence_block_partial_signature_created(
                        block_hash,
                        public_key_share,
                        partial_signature,
                    );
            },
            Event::ConvergenceBlockPeerSignatureRequested {
                node_id,
                block_hash,
                public_key_share,
                partial_signature,
            } => {
                self.consensus_driver
                    .handle_convergence_block_peer_signature_request(
                        node_id,
                        block_hash,
                        public_key_share,
                        partial_signature,
                    );
            },
            Event::ConvergenceBlockPrecheckRequested {
                convergence_block,
                block_header,
            } => {
                let resolver = self.mining_driver.clone();
                self.handle_convergence_block_precheck_requested(convergence_block, block_header, resolver);
            },

            Event::TxnsReadyForProcessing(txns) => {
                // Receives a batch of transactions from mempool and sends
                // them to scheduler to get it validated and voted
                self.consensus_driver.handle_txns_ready_for_processing(txns);
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
                //                if let Ok(account) =
                // decode_from_binary_byte_slice(&account_bytes) {
                // self.update_account(address, account)
                // .map_err(|err| TheaterError::Other(err.to_string()))?;
                //               }
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
                let next_event = self.state_driver
                    .handle_block_received(&mut block)
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                self.events_tx.send(next_event.into()).await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },
            Event::HarvesterSignatureReceived(block_hash, node_id, sig) => {
                // TODO, refactor into a node_runtime method
                self.handle_harvester_signature_received(
                    block_hash, 
                    node_id, 
                    sig
                ).await.map_err(|err| TheaterError::Other(err.to_string()))?;
            },
            Event::BlockCertificateCreated(certificate) => {
                self.handle_block_certificate_created(certificate)
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },
            Event::BlockConfirmed(certificate) => {
                let certificate: Certificate = bincode::deserialize(&certificate)
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                self.handle_block_certificate(certificate).await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            }
            Event::QuorumFormed => {
                self.handle_quorum_formed().await.map_err(|err| {
                    TheaterError::Other(err.to_string())
                })?;
            },
            Event::QuorumMembersReceived(quorum_members) => self
                .state_driver
                .handle_quorum_members_received(quorum_members),
            // Event::ElectedMiner((_winner_claim_hash, winner_claim)) => {
            //     if self.miner.check_claim(winner_claim.hash) {
            //         let mining_result = self.miner.try_mine();
            //
            //         if let Ok(block) = mining_result {
            //             let _ = self
            //                 .events_tx
            //                 .send(Event::MinedBlock(block.clone()).into())
            //                 .await
            //                 .map_err(|err| {
            //                     theater::TheaterError::Other(format!(
            //                         "failed to send mined block to event bus: {err}"
            //                     ))
            //                 });
            //         }
            //     };
            // },
            // Event::CheckConflictResolution((proposal_blocks, round, seed, convergence_block)) =>
            // {     let tmp_proposal_blocks = proposal_blocks.clone();
            //     let resolved_proposals_set = self
            //         .miner
            //         .resolve(&tmp_proposal_blocks, round, seed)
            //         .iter()
            //         .cloned()
            //         .collect::<HashSet<ProposalBlock>>();
            //
            //     let proposal_blocks_set = proposal_blocks
            //         .iter()
            //         .cloned()
            //         .collect::<HashSet<ProposalBlock>>();
            //
            //     if proposal_blocks_set == resolved_proposals_set {
            //         if let Err(err) = self
            //             .events_tx
            //             .send(EventMessage::new(
            //                 None,
            //                 Event::SignConvergenceBlock(convergence_block),
            //             ))
            //             .await
            //         {
            //             theater::TheaterError::Other(format!(
            //                 "failed to send EventMessage for Event::SignConvergenceBlock: {err}"
            //             ));
            //         };
            //     }
            // },
            Event::TxnAddedToMempool(txn_hash) => {
                let mempool_reader = self.mempool_read_handle_factory().clone();
                let state_reader = self.state_store_read_handle_factory().clone();
                if let Ok((transaction, validity)) = self.validate_transaction_kind(
                    txn_hash,
                    mempool_reader,
                    state_reader
                ) {
                    if let Ok(vote) = self.cast_vote_on_transaction_kind(transaction, validity) {
                        self.events_tx
                            .send(
                                Event::TransactionsValidated {
                                    vote,
                                    quorum_threshold: self.config.threshold_config.threshold as usize,
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
                self.events_tx.send(
                    Event::BroadcastTransactionVote(vote).into()
                ).await.map_err(|err| TheaterError::Other(err.to_string()))?;
            },
            Event::NoOp => {},
            _ => {},
        }

        Ok(ActorState::Running)
    }
}
