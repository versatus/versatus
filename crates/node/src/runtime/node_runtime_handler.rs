use crate::{node_runtime::NodeRuntime, NodeError};
use async_trait::async_trait;
use block::{Block, GenesisReceiver};
use events::{AssignedQuorumMembership, Event, EventMessage};
use primitives::{Address, BlockPartialSignature, NodeType, QuorumKind};
use telemetry::{error, info, warn};
use theater::{ActorId, ActorLabel, ActorState, Handler};
use vrrb_core::transactions::Transaction;

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
                                error!("failed to send quorum assignments to network: {:?}", err);
                            }
                        }
                    }
                    Err(err) => error!("failed to add node to peer list: {:?}", err),
                }
            }

            //
            // ==============================================================================================================
            // BOOTSTRAP HANDLERS
            // ==============================================================================================================
            //
            // TODO: consider eliminating this match arm and bundling this logic with the arm above
            Event::QuorumMembershipAssigmentsCreated(assignments) => {
                if let Err(err) = self.handle_quorum_membership_assigments_created(assignments) {
                    error!("failed to create quorum membership assignments: {:?}", err);
                    return Ok(ActorState::Running);
                }
                let own_node_id = self.config_ref().id.clone();

                match self
                    .consensus_driver
                    .quorum_kind
                    .to_owned()
                    .ok_or(NodeError::Other(format!(
                        "Node {} has no quorum kind set",
                        own_node_id
                    ))) {
                    Ok(quorum_kind) => {
                        // TODO: write test case to ensure that no two miners are allowed to mine a genesis
                        // block

                        //get the lowest node_id with node_type == NodeType::Miner
                        match self
                            .config_ref()
                            .whitelisted_nodes
                            .iter()
                            .filter(|quorum_member| quorum_member.node_type == NodeType::Miner)
                            .min_by(|a, b| a.node_id.cmp(&b.node_id))
                        {
                            Some(first_miner) => {
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
                                            GenesisReceiver::new(Address::new(
                                                quorum_member.validator_public_key,
                                            ))
                                        })
                                        .collect(),
                                };

                                if let Err(err) = self.send_event_to_self(content).await {
                                    error!("failed to elect genesis miner: {:?}", err);
                                };
                            }
                            None => {
                                error!("no miners found in quorum");
                            }
                        }
                    }
                    Err(err) => {
                        error!("{:?}", err);
                    }
                }
            }
            Event::QuorumMembershipAssigmentCreated(assigned_membership) => {
                if let Err(err) =
                    self.handle_quorum_membership_assigment_created(assigned_membership.clone())
                {
                    error!("failed to find assigned membership: {:?}", err);
                };
            }

            // ==============================================================================================================
            // MINER HANDLERS
            // ==============================================================================================================
            //
            Event::GenesisMinerElected { genesis_receivers } => {
                match self.distribute_genesis_reward(genesis_receivers) {
                    Ok(genesis_rewards) => {
                        match self.mine_genesis_block(genesis_rewards) {
                            Ok(block) => {
                                if let Err(err) = self
                                    .send_event_to_network(Event::GenesisBlockCreated(
                                        block.clone(),
                                    ))
                                    .await
                                {
                                    error!("failed to send event to network: {:?}", err);
                                    return Ok(ActorState::Running);
                                };
                                if let Err(err) = self
                                    .send_event_to_self(Event::GenesisBlockCreated(block))
                                    .await
                                {
                                    error!("failed to send event to self: {:?}", err);
                                };
                            }
                            Err(err) => {
                                error!("failed to mine genesis block: {:?}", err);
                            }
                        };
                    }
                    Err(err) => {
                        error!("failed to distribute to genesis receivers: {:?}", err);
                    }
                }
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

                match self.handle_genesis_block_received(block) {
                    Ok(next_event) => {
                        if let Err(err) = self.send_event_to_self(next_event).await {
                            error!("failed to send event to self: {:?}", err);
                        }
                    }
                    Err(err) => {
                        error!("error handling genesis block: {:?}", err);
                    }
                }
            }
            Event::ProposalBlockCreated(block) => {
                let node_id = self.config_ref().id.clone();
                telemetry::info!(
                    "Node {} received proposal block from network: {:?}",
                    node_id,
                    block.hash
                );

                match self.handle_proposal_block_received(block) {
                    Ok(next_event) => {
                        if let Err(err) = self.send_event_to_network(next_event).await {
                            error!("failed to send event to network: {:?}", err);
                        }
                    }
                    Err(err) => {
                        error!("error handling genesis block: {:?}", err);
                    }
                };
            }
            Event::ConvergenceBlockCreated(block) => {
                let node_id = self.config_ref().id.clone();
                telemetry::info!(
                    "Node {} received convergence block from network: {}",
                    node_id,
                    block.hash
                );

                match self.handle_convergence_block_received(block) {
                    Ok(next_event) => {
                        if let Err(err) = self.send_event_to_network(next_event).await {
                            error!("failed to send event to network: {:?}", err);
                        }
                    }
                    Err(err) => {
                        error!("error handling genesis block: {:?}", err);
                    }
                };
            }
            // Triggered once a genesis block or convergence block makes it to state
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
                            if let Err(err) = self.send_event_to_self(event).await {
                                error!("failed to send genesis block signature to self: {:?}", err);
                            };
                        }
                        Block::Convergence { block } => {
                            let event = Event::ConvergenceBlockSignatureRequested(block);
                            if let Err(err) = self.send_event_to_self(event).await {
                                error!(
                                    "failed to send convergence block signature to self: {:?}",
                                    err
                                );
                            };
                        }
                        _ => error!("error updating status for block: {:?}", block),
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
                if let Err(err) = self.handle_convergence_block_certificate_created(certificate) {
                    error!("error creating convergence block certificate: {:?}", err);
                };
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

                if let Err(err) = self
                    .send_event_to_network(Event::NewTxnForwarded(
                        self.config_ref().id.clone(),
                        txn.clone(),
                    ))
                    .await
                {
                    error!("error sending event to network: {:?}", err);
                    return Ok(ActorState::Running);
                };

                info!("Transaction {} broadcast to network", txn.id().to_string());

                match self.state_driver.insert_txn_to_mempool(txn) {
                    Ok(txn_hash) => {
                        if let Err(err) = self
                            .send_event_to_self(Event::TxnAddedToMempool(txn_hash.clone()))
                            .await
                        {
                            error!("error sending event to self: {:?}", err);
                        }
                    }
                    Err(err) => {
                        error!("failure to insert txn into mempool: {:?}", err);
                    }
                }
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

                match self.state_driver.insert_txn_to_mempool(txn) {
                    Ok(txn_hash) => {
                        if let Err(err) = self
                            .send_event_to_self(Event::TxnAddedToMempool(txn_hash.clone()))
                            .await
                        {
                            error!("error sending event to self: {:?}", err);
                        }
                    }
                    Err(err) => {
                        error!("failure to insert txn into mempool: {:?}", err);
                    }
                }
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
                        if let Err(err) = self
                            .send_event_to_network(Event::TransactionVoteCreated(vote))
                            .await
                        {
                            error!("failed to send event to network: {:?}", err);
                        };
                    }
                    Err(err) => {
                        error!("failed to create vote: {}", err);
                    }
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
                if let Err(err) = self
                    .send_event_to_network(Event::TransactionVoteForwarded(vote.clone()))
                    .await
                {
                    error!("failed to send event to network: {:?}", err);
                    return Ok(ActorState::Running);
                };

                if let Err(err) = self.handle_vote_received(vote).await {
                    error!("failed to handle vote for {}: {}", txn_id, err);
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
                    error!("failed to handle vote for {}: {}", txn_id, err);
                }
            }
            Event::TxnValidated(txn) => {
                if let Err(err) = self.state_driver.handle_transaction_validated(txn).await {
                    error!("failed to validate txn: {:?}", err);
                };
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
                if let Err(err) = self.handle_quorum_election_started(header) {
                    error!("error starting quorum election: {:?}", err);
                };
            }
            Event::MinerElectionStarted(header) => {
                match self.state_driver.read_handle().claim_store_values() {
                    Ok(claims) => match self
                        .consensus_driver
                        .handle_miner_election_started(header, claims)
                    {
                        Ok(results) => match results.clone().into_iter().next() {
                            Some(winner) => {
                                if let Err(err) = self
                                    .send_event_to_network(Event::MinerElected(winner))
                                    .await
                                {
                                    error!("error sending event to network: {:?}", err);
                                }
                            }
                            None => {
                                error!("found no winner");
                            }
                        },
                        Err(err) => {
                            error!("error retrieving results from claim: {:?}", err);
                        }
                    },
                    Err(err) => {
                        error!("error reading claims: {:?}", err);
                    }
                };
            }
            Event::ConvergenceBlockPrecheckRequested {
                convergence_block,
                block_header,
            } => {
                let resolver = self.mining_driver.clone();
                if let Err(err) = self
                    .handle_convergence_block_precheck_requested(
                        convergence_block,
                        block_header,
                        resolver,
                    )
                    .await
                {
                    error!("error in convergence block precheck: {:?}", err);
                };
            }
            Event::GenesisBlockSignatureRequested(block) => {
                let block_hash = block.hash.clone();
                let signature = match self.handle_sign_genesis_block(&block) {
                    Ok(signature) => signature,
                    Err(err) => {
                        error!("error signing block: {}", err);
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

                if let Err(err) = self.send_event_to_network(event.clone()).await {
                    error!("error sending event to network: {:?}", err);
                    return Ok(ActorState::Running);
                };

                if let Err(err) = self.send_event_to_self(event).await {
                    error!("error sending event to self: {:?}", err);
                };
            }
            Event::ConvergenceBlockSignatureRequested(block) => {
                let block_hash = block.hash.clone();
                let signature = match self.handle_sign_convergence_block(&block) {
                    Ok(signature) => signature,
                    Err(err) => {
                        error!("error signing block: {:?}", err);
                        return Ok(ActorState::Running);
                    }
                };

                telemetry::info!("Node {} signed block: {}", self.config_ref().id, block_hash);

                let partial_signature = BlockPartialSignature {
                    node_id: self.config_ref().id.clone(),
                    signature,
                    block_hash,
                };

                if let Err(err) = self
                    .send_event_to_network(Event::ConvergenceBlockSignatureCreated(
                        partial_signature,
                    ))
                    .await
                {
                    error!("error sending event to network: {:?}", err);
                };
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
                    .await;

                match certificate {
                    Ok(certificate) => {
                        info!("certificate created");

                        if let Err(err) = self
                            .send_event_to_network(Event::GenesisBlockCertificateCreated(
                                certificate,
                            ))
                            .await
                        {
                            error!("error sending event to network: {:?}", err);
                        };
                    }
                    Err(err) => {
                        error!("certificate not created: {:?}", err);
                    }
                }
            }

            Event::ConvergenceBlockSignatureCreated(BlockPartialSignature {
                block_hash,
                signature,
                node_id,
            }) => {
                match self
                    .handle_harvester_signature_received(block_hash, node_id, signature)
                    .await
                {
                    Ok(certificate) => {
                        if let Err(err) = self
                            .send_event_to_network(Event::ConvergenceBlockCertificateCreated(
                                certificate,
                            ))
                            .await
                        {
                            error!("error sending event to network: {:?}", err);
                        }
                    }
                    Err(err) => {
                        error!("error creating convergence block signature: {:?}", err);
                    }
                };
            }

            Event::GenesisBlockCertificateCreated(certificate) => {
                info!("GenesisBlockCertificateCreated");
                match self.handle_genesis_block_certificate_created(certificate) {
                    Ok(confirmed_block) =>
                    // TODO: update state after this
                    {
                        if let Err(err) = self
                            .send_event_to_self(Event::UpdateState(confirmed_block))
                            .await
                        {
                            error!("error sending event to self: {:?}", err);
                        }
                    }
                    Err(err) => {
                        error!("error creating genesis block certificate: {:?}", err);
                    }
                };
            }

            Event::ConvergenceBlockCertificateCreated(certificate) => {
                match self.handle_convergence_block_certificate_created(certificate) {
                    Ok(confirmed_block) =>
                    // TODO: update state after this
                    {
                        if let Err(err) = self
                            .send_event_to_self(Event::UpdateState(confirmed_block))
                            .await
                        {
                            error!("error sending event to self: {:?}", err);
                        }
                    }
                    Err(err) => {
                        error!("error creating convergence block certificate: {:?}", err);
                    }
                };
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
            Event::QuorumFormed => {
                if let Err(err) = self.handle_quorum_formed().await {
                    error!("error forming quorum: {:?}", err);
                }
            }
            _ => {}
        }

        Ok(ActorState::Running)
    }
}
