use crate::{node_runtime::NodeRuntime, NodeError};
use async_trait::async_trait;
use block::{Block, Certificate, GenesisReceiver};
use events::{AssignedQuorumMembership, Event, EventMessage};
use primitives::{
    Address, BlockPartialSignature, ConvergencePartialSig, NodeType, QuorumKind, NETWORK_TOPIC_STR,
    RUNTIME_TOPIC_STR,
};
use telemetry::{error, info, warn};
use theater::{ActorId, ActorLabel, ActorState, Handler, TheaterError};
use vrrb_config::QuorumMember;
use vrrb_core::{ownable, transactions::Transaction};

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

    // fn on_error(&self, err: TheaterError) {
    //     dbg!(&err);
    //     telemetry::error!("{}", err);
    // }

    fn on_start(&self) {
        info!("{} starting", self.label());
    }

    fn on_stop(&self) {
        info!("{} received stop signal. Stopping", self.label());
    }

    async fn handle(&mut self, event: EventMessage) -> theater::Result<ActorState> {
        match event.into() {
            //
            // ==============================================================================================================
            // EVERYONE HANDLERS
            // ==============================================================================================================
            //
            Event::NodeAddedToPeerList(peer_data) => {
                match self.handle_node_added_to_peer_list(peer_data.clone()).await {
                    Ok(assignments) => {
                        if let Some(assignments) = assignments {
                            let assignments = assignments
                                .into_values()
                                .collect::<Vec<AssignedQuorumMembership>>();

                            if let Err(err) = self
                                .send_event_to_network(Event::QuorumMembershipAssigmentsCreated(
                                    assignments,
                                ))
                                .await
                            {
                                error!("failed to send quorum assignments to network: {}", err);
                            }
                        }
                    }
                    Err(err) => error!("failed to add node to peer list: {}", err),
                }
            }

            //
            // ==============================================================================================================
            // BOOTSTRAP HANDLERS
            // ==============================================================================================================
            //
            // TODO: consider eliminating this match arm and bundling this logic with the arm above
            Event::QuorumMembershipAssigmentsCreated(assignments) => {
                self.handle_quorum_membership_assigments_created(assignments)?;
                let own_node_id = self.config_ref().id.clone();

                let quorum_kind =
                    self.consensus_driver
                        .quorum_kind
                        .to_owned()
                        .ok_or(NodeError::Other(format!(
                            "Node {} has no quorum kind set",
                            own_node_id
                        )))?;

                // TODO: write test case to ensure that no two miners are allowed to mine a genesis
                // block

                //get the lowest node_id with node_type == NodeType::Miner
                let first_miner: &QuorumMember = self
                    .config_ref()
                    .whitelisted_nodes
                    .iter()
                    .filter(|quorum_member| quorum_member.node_type == NodeType::Miner)
                    .min_by(|a, b| a.node_id.cmp(&b.node_id))
                    .expect("No miners found in quorum");

                let is_chosen_miner = own_node_id == first_miner.node_id;

                let can_mine_genblock = quorum_kind == QuorumKind::Miner
                    && self.config.node_type == NodeType::Miner
                    && is_chosen_miner;

                if !can_mine_genblock {
                    // TODO: consider logging a debug message here
                    return Ok(ActorState::Running);
                }

                let content = Event::GenesisMinerElected {
                    genesis_receivers: self
                        .config
                        .whitelisted_nodes
                        .iter()
                        .map(|quorum_member| {
                            GenesisReceiver::new(Address::new(quorum_member.validator_public_key))
                        })
                        .collect(),
                };

                self.send_event_to_self(content).await?;
            }
            Event::QuorumMembershipAssigmentCreated(assigned_membership) => {
                self.handle_quorum_membership_assigment_created(assigned_membership.clone())?;
            }

            // ==============================================================================================================
            // MINER HANDLERS
            // ==============================================================================================================
            //
            Event::GenesisMinerElected { genesis_receivers } => {
                let genesis_rewards = self.distribute_genesis_reward(genesis_receivers)?;

                let block = self.mine_genesis_block(genesis_rewards)?;

                self.send_event_to_network(Event::GenesisBlockCreated(block.clone()))
                    .await?;

                self.send_event_to_self(Event::GenesisBlockCreated(block))
                    .await?;
            }
            // Event::BlockCreated(block) => {
            //     let node_id = self.config_ref().id.clone();
            //     telemetry::info!(
            //         "Node {} received block from network: {}",
            //         node_id,
            //         block.hash()
            //     );
            //
            //     let next_event = self.handle_block_received(block)?;
            //
            //     self.send_event_to_network(next_event).await?;
            // }
            Event::GenesisBlockCreated(block) => {
                let node_id = self.config_ref().id.clone();
                telemetry::info!(
                    "Node {} received genesis block from network: {}",
                    node_id,
                    block.hash
                );

                let next_event = match self.handle_genesis_block_received(block) {
                    Ok(event) => event,
                    Err(err) => {
                        info!("error handling genesis block: {}", err);
                        return Ok(ActorState::Running);
                    }
                    _ => {
                        info!("error handling genesis block");
                        return Ok(ActorState::Running);
                    }
                };

                self.send_event_to_self(next_event).await?;
            }
            Event::ProposalBlockCreated(block) => {
                let node_id = self.config_ref().id.clone();
                telemetry::info!(
                    "Node {} received proposal block from network: {}",
                    node_id,
                    block.hash
                );

                let next_event = self.handle_proposal_block_received(block)?;

                self.send_event_to_network(next_event).await?;
            }
            Event::ConvergenceBlockCreated(block) => {
                let node_id = self.config_ref().id.clone();
                telemetry::info!(
                    "Node {} received convergence block from network: {}",
                    node_id,
                    block.hash
                );

                let next_event = self.handle_convergence_block_received(block)?;

                self.send_event_to_network(next_event).await?;
            }
            /// Triggered once a genesis block or convergence block makes it to state
            // TODO: create a specific handler for this event
            Event::StateUpdated(block) => {
                if self.is_harvester().is_ok() {
                    info!(
                        "StateUpdated Node {} is a harvester, block {}",
                        self.config_ref().id,
                        block.hash()
                    );
                    match block {
                        Block::Genesis { block } => {
                            // send certificate requested to self
                            let event = Event::GenesisBlockSignatureRequested(block);
                            self.send_event_to_self(event).await?;
                        }
                        Block::Convergence { block } => {
                            let event = Event::ConvergenceBlockSignatureRequested(block);
                            self.send_event_to_self(event).await?;
                        }
                        _ => {}
                    }
                }

                // if let Err(err) = self.state_driver.update_state(block.hash) {
                //     telemetry::error!("error updating state: {}", err);
                // } else {
                //     self.events_tx
                //         .send(Event::BuildProposalBlock(block).into())
                //         .await
                //         .map_err(|err| TheaterError::Other(err.to_string()))?;
                // }
            }
            // Event::UpdateState(block) => {
            //     if let Err(err) = self.state_driver.update_state(block.hash.clone()) {
            //         telemetry::error!("error updating state: {}", err);
            //     } else {
            //         self.events_tx
            //             .send(Event::BuildProposalBlock(block).into())
            //             .await
            //             .map_err(|err| TheaterError::Other(err.to_string()))?;
            //     }
            // }
            Event::ConvergenceBlockCertificateCreated(certificate) => {
                self.handle_convergence_block_certificate_created(certificate)?;
            }
            Event::ConvergenceBlockCertified(certified_block) => {
                //
            }
            //
            //
            //
            // ===============================================================================================================
            // ===============================================================================================================
            // ===============================================================================================================
            // ===============================================================================================================
            //
            //
            //
            //
            // ==============================================================================================================
            // FARMER HANDLERS
            // ==============================================================================================================
            //
            // Event::NewTxnCreated(txn) => {
            //     let txn_hash = self
            //         .state_driver
            //         .handle_new_txn_created(txn)
            //         .map_err(|err| TheaterError::Other(err.to_string()))?;
            //
            //     self.events_tx
            //         .send(Event::TxnAddedToMempool(txn_hash.clone()).into())
            //         .await
            //         .map_err(|err| TheaterError::Other(err.to_string()))?;
            // },
            //
            //
            Event::NewTxnCreated(txn) => {
                info!(
                    "Node {} received transaction: {}",
                    &self.config_ref().id,
                    txn.id().to_string(),
                );

                let is_txn_in_mempool = self
                    .state_driver
                    .read_handle()
                    .transaction_store_values()
                    .unwrap_or_default()
                    .contains_key(&txn.id());

                // check for txn in mempool, return if present
                if is_txn_in_mempool {
                    warn!("Transaction {} already in mempool", txn.id().to_string());
                    return Ok(ActorState::Running);
                }

                info!(
                    "Broadcasting {} to network from node {}",
                    txn.id().to_string(),
                    self.config_ref().id
                );

                self.send_event_to_network(Event::NewTxnForwarded(
                    self.config_ref().id.clone(),
                    txn.clone(),
                ))
                .await?;

                info!("Transaction {} broadcast to network", txn.id().to_string());

                let txn_hash = self.state_driver.insert_txn_to_mempool(txn)?;

                self.send_event_to_self(Event::TxnAddedToMempool(txn_hash.clone()))
                    .await?;
            }
            Event::NewTxnForwarded(node_id, txn) => {
                info!(
                    "Node {} received transaction: {}",
                    &self.config_ref().id,
                    txn.id().to_string(),
                );

                println!(
                    "Node {} received transaction {}",
                    &self.config_ref().id,
                    txn.id().to_string(),
                );

                info!(
                    "Broadcasting {} to network from node {}",
                    txn.id().to_string(),
                    self.config_ref().id
                );

                let txn_hash = self.state_driver.insert_txn_to_mempool(txn)?;

                self.send_event_to_self(Event::TxnAddedToMempool(txn_hash.clone()))
                    .await?;
            }
            Event::TxnAddedToMempool(txn_hash) => {
                // check to see if txn is already in mempool, return if present
                info!(
                    "Node {} added transaction with hash {} to mempool",
                    &self.config_ref().id,
                    txn_hash.digest_string()
                );

                let vote = self.handle_txn_added_to_mempool(txn_hash);

                match vote {
                    Ok(vote) => {
                        self.send_event_to_network(Event::TransactionVoteCreated(vote))
                            .await?;
                    }
                    Err(err) => {
                        telemetry::error!("failed to create vote: {}", err);
                    }
                    _ => {}
                }
            }
            Event::TransactionVoteCreated(vote) => {
                let txn_id = vote.txn.id();
                info!(
                    "Node runtime {} received vote for {} from {}",
                    self.config_ref().id,
                    txn_id,
                    vote.farmer_id
                );

                //forward vote to other network nodes
                self.send_event_to_network(Event::TransactionVoteForwarded(vote.clone()))
                    .await?;

                if let Err(err) = self.handle_vote_received(vote).await {
                    telemetry::error!("failed to handle vote for {}: {}", txn_id, err);
                }
            }
            Event::TransactionVoteForwarded(vote) => {
                let txn_id = vote.txn.id();
                info!(
                    "Node runtime for {} received forwarded vote for {} from {}",
                    self.config_ref().id,
                    txn_id,
                    vote.farmer_id
                );

                if let Err(err) = self.handle_vote_received(vote).await {
                    telemetry::error!("failed to handle vote for {}: {}", txn_id, err);
                }
            }
            Event::TxnValidated(txn) => {
                self.state_driver.handle_transaction_validated(txn).await?;
            }
            Event::BuildProposalBlock() => {
                // let proposal_block = self.handle_build_proposal_block_requested().await?;
            }
            //
            // ==============================================================================================================
            // HARVESTER HANDLERS
            // ==============================================================================================================
            //
            Event::QuorumElectionStarted(header) => {
                self.handle_quorum_election_started(header)?;
            }
            Event::MinerElectionStarted(header) => {
                let claims = self
                    .state_driver
                    .read_handle()
                    .claim_store_values()
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                let results = self
                    .consensus_driver
                    .handle_miner_election_started(header, claims)?;

                let winner = results
                    .clone()
                    .into_iter()
                    .next()
                    .ok_or(TheaterError::Other("no winner found".to_string()))?;

                self.send_event_to_network(Event::MinerElected(winner))
                    .await?;
            }
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
                .await?;
            }
            Event::GenesisBlockSignatureRequested(block) => {
                let block_hash = block.hash.clone();
                let signature = match self.handle_sign_genesis_block(&block) {
                    Ok(signature) => signature,
                    Err(err) => {
                        telemetry::error!("error signing block: {}", err);
                        return Ok(ActorState::Running);
                    }
                };

                info!("Node {} signed block: {}", self.config_ref().id, block_hash);

                let partial_signature = BlockPartialSignature {
                    node_id: self.config_ref().id.clone(),
                    signature,
                    block_hash,
                };

                let event = Event::GenesisBlockSignatureCreated(partial_signature);

                self.send_event_to_network(event.clone()).await?;

                self.send_event_to_self(event).await?;
            }
            Event::ConvergenceBlockSignatureRequested(block) => {
                let block_hash = block.hash.clone();
                let signature = self.handle_sign_convergence_block(&block)?;

                telemetry::info!("Node {} signed block: {}", self.config_ref().id, block_hash);

                let partial_signature = BlockPartialSignature {
                    node_id: self.config_ref().id.clone(),
                    signature,
                    block_hash,
                };

                self.send_event_to_network(Event::ConvergenceBlockSignatureCreated(
                    partial_signature,
                ))
                .await?;
            }

            Event::GenesisBlockSignatureCreated(BlockPartialSignature {
                block_hash,
                signature,
                node_id,
            }) => {
                info!(
                    "handling GenesisBlockSignatureCreated on {}, a {}, from {}",
                    self.config_ref().id,
                    self.config_ref().node_type,
                    node_id
                );
                let certificate = self
                    .handle_harvester_signature_received(block_hash, node_id, signature)
                    .await?;

                info!("certificate created");

                self.send_event_to_network(Event::GenesisBlockCertificateCreated(certificate))
                    .await?;
            }

            Event::ConvergenceBlockSignatureCreated(BlockPartialSignature {
                block_hash,
                signature,
                node_id,
            }) => {
                let certificate = self
                    .handle_harvester_signature_received(block_hash, node_id, signature)
                    .await?;

                self.send_event_to_network(Event::ConvergenceBlockCertificateCreated(certificate))
                    .await?;
            }

            Event::GenesisBlockCertificateCreated(certificate) => {
                info!("GenesisBlockCertificateCreated");
                let confirmed_block = self.handle_genesis_block_certificate_created(certificate)?;

                // TODO: update state after this
                self.send_event_to_self(Event::UpdateState(confirmed_block))
                    .await?;
            }

            Event::ConvergenceBlockCertificateCreated(certificate) => {
                let confirmed_block =
                    self.handle_convergence_block_certificate_created(certificate)?;

                // TODO: update state after this
                self.send_event_to_self(Event::UpdateState(confirmed_block))
                    .await?;
            }

            // Event::BlockConfirmed(cert_bytes) => {
            //     let certificate: Certificate = bincode::deserialize(&cert_bytes)
            //         .map_err(|err| TheaterError::Other(err.to_string()))?;
            //
            //     let confirmed_block = self
            //         .handle_convergence_block_certificate_received(certificate)
            //         .await
            //         .map_err(|err| TheaterError::Other(err.to_string()))?;
            //
            //     self.events_tx
            //         .send(Event::UpdateState(confirmed_block).into())
            //         .await
            //         .map_err(|err| TheaterError::Other(err.to_string()))?;
            // },

            //
            // ==============================================================================================================
            // MISC HANDLERS
            // ==============================================================================================================
            //
            Event::ClaimReceived(claim) => {
                info!("Storing claim from: {}", claim.address);
                // Claim should be added to pending claims
                // Event to validate claim should be created
            }
            Event::QuorumFormed => self.handle_quorum_formed().await?,
            _ => {}
        }

        Ok(ActorState::Running)
    }
}
