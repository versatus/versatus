use crate::{node_runtime::NodeRuntime, NodeError};
use async_trait::async_trait;
use block::{Block, Certificate, GenesisReceiver};
use events::{AssignedQuorumMembership, Event, EventMessage};
use primitives::{
    Address, BlockPartialSignature, ConvergencePartialSig, NodeType, QuorumKind, NETWORK_TOPIC_STR,
    RUNTIME_TOPIC_STR,
};
use telemetry::info;
use theater::{ActorId, ActorLabel, ActorState, Handler, TheaterError};
use vrrb_core::ownable;

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
                let assignments = self
                    .handle_node_added_to_peer_list(peer_data.clone())
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                if let Some(assignments) = assignments {
                    let assignments = assignments
                        .into_values()
                        .collect::<Vec<AssignedQuorumMembership>>();

                    let event = EventMessage::new(
                        Some(NETWORK_TOPIC_STR.into()),
                        Event::QuorumMembershipAssigmentsCreated(assignments),
                    );

                    self.events_tx
                        .send(event)
                        .await
                        .map_err(|err| TheaterError::Other(err.to_string()))?;
                }
            },

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

                let is_chosen_miner = self
                    .config_ref()
                    .whitelisted_nodes
                    .iter()
                    .map(|quorum_member| (&quorum_member.node_id, &quorum_member.node_type))
                    .any(|(id, node_type)| id == &own_node_id && node_type == &NodeType::Miner);

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

                let event = EventMessage::new(Some(RUNTIME_TOPIC_STR.into()), content);

                self.events_tx
                    .send(event)
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },
            Event::QuorumMembershipAssigmentCreated(assigned_membership) => {
                self.handle_quorum_membership_assigment_created(assigned_membership.clone())?;
            },

            // ==============================================================================================================
            // MINER HANDLERS
            // ==============================================================================================================
            //
            Event::GenesisMinerElected { genesis_receivers } => {
                let genesis_rewards = self
                    .distribute_genesis_reward(genesis_receivers)
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                let block = self
                    .mine_genesis_block(genesis_rewards)
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                self.send_event_to_network(Event::BlockCreated(Block::Genesis {
                    block: block.clone(),
                }))
                .await?;

                self.send_event_to_self(Event::BlockCreated(Block::Genesis { block }))
                    .await?;
            },
            Event::BlockCreated(block) => {
                let node_id = self.config_ref().id.clone();
                telemetry::info!(
                    "Node {} received block from network: {}",
                    node_id,
                    block.hash()
                );

                let next_event = self
                    .handle_block_received(block)
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                let em = EventMessage::new(Some(NETWORK_TOPIC_STR.into()), next_event);

                self.events_tx
                    .send(em)
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },
            Event::GenesisBlockCertificateRequested {
                genesis_block,
                block_header,
            } => {
                //
                // TODO: refactor other handlers to provide informative logs
                let quorum_kind = self
                    .quorum_membership()
                    .ok_or(TheaterError::Other(format!(
                        "No membership information found for node {}",
                        &self.config.id
                    )))?
                    .quorum_kind;

                if quorum_kind != QuorumKind::Harvester {
                    telemetry::warn!(
                        "Block certification requested, but node {} is not a harvester",
                        &self.config.id
                    );
                    return Ok(ActorState::Running);
                }

                let certificate = self
                    .certify_genesis_block(
                        genesis_block.clone(),
                        self.config.id.clone(),
                        block_header.miner_signature,
                    )
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                let em = EventMessage::new(
                    Some(NETWORK_TOPIC_STR.into()),
                    Event::GenesisBlockCertificateCreated(certificate),
                );

                self.events_tx
                    .send(em)
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },
            Event::GenesisBlockCertificateCreated(certificate) => {
                let block_hash = certificate.block_hash.clone();
                let confirmed_block = self
                    .handle_genesis_block_certificate_received(&block_hash, certificate)
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                if let Err(err) = self.update_state(Block::from(confirmed_block.clone())) {
                    telemetry::error!("error updating state: {}", err);
                }

                let em = EventMessage::new(
                    Some(NETWORK_TOPIC_STR.into()),
                    Event::StateUpdated(Block::from(confirmed_block)),
                );

                self.events_tx
                    .send(em)
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },
            Event::StateUpdated(block) => {
                // if let Err(err) = self.state_driver.update_state(block.hash.clone()) {
                //     telemetry::error!("error updating state: {}", err);
                // } else {
                //     self.events_tx
                //         .send(Event::BuildProposalBlock(block).into())
                //         .await
                //         .map_err(|err| TheaterError::Other(err.to_string()))?;
                // }
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
            Event::TxnAddedToMempool(txn_hash) => {
                // if this txn exists on my mempool
                //     return early
                //
                // if havent seen this txn bfore, broadcast it

                let vote = self
                    .handle_txn_added_to_mempool(txn_hash)
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                let em = EventMessage::new(
                    Some(NETWORK_TOPIC_STR.into()),
                    Event::BroadcastTransactionVote(vote),
                );

                self.events_tx
                    .send(em)
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },
            Event::TxnValidated(txn) => {
                self.state_driver.handle_transaction_validated(txn).await?;
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
            //
            // ==============================================================================================================
            // HARVESTER HANDLERS
            // ==============================================================================================================
            //
            Event::QuorumElectionStarted(header) => {
                self.handle_quorum_election_started(header)
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },
            Event::MinerElectionStarted(header) => {
                let claims = self
                    .state_driver
                    .read_handle()
                    .claim_store_values()
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

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
            Event::BlockSignatureRequested(block) => {
                let block_hash = block.hash();
                let signature = self
                    .handle_sign_block(block)
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                telemetry::info!("Node {} signed block: {}", self.config_ref().id, block_hash);

                let partial_signature = BlockPartialSignature {
                    node_id: self.config_ref().id.clone(),
                    signature,
                    block_hash,
                };

                let em = EventMessage::new(
                    Some(NETWORK_TOPIC_STR.into()),
                    Event::BlockSignatureCreated(partial_signature),
                );

                self.events_tx
                    .send(em)
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },

            Event::BlockSignatureCreated(BlockPartialSignature {
                block_hash,
                signature,
                node_id,
            }) => {
                let certificate = self
                    .handle_harvester_signature_received(block_hash, node_id, signature)
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                let em = EventMessage::new(
                    Some(NETWORK_TOPIC_STR.into()),
                    Event::BlockCertificateCreated(certificate),
                );

                self.events_tx
                    .send(em)
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },
            // NOTE: replaced by handler above
            // Event::HarvesterSignatureReceived(block_hash, node_id, sig) => {
            //     self.handle_harvester_signature_received(block_hash, node_id, sig)
            //         .await
            //         .map_err(|err| TheaterError::Other(err.to_string()))?;
            // },
            Event::GenesisBlockCertificateCreated(certificate) => {
                let confirmed_block = self
                    .handle_genesis_block_certificate_created(certificate)
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },
            Event::ConvergenceBlockCertificateCreated(certificate) => {
                let confirmed_block = self
                    .handle_convergence_block_certificate_created(certificate)
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                // TODO: update state after this

                self.events_tx
                    .send(Event::UpdateState(confirmed_block).into())
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },

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
            },
            Event::QuorumFormed => self
                .handle_quorum_formed()
                .await
                .map_err(|err| TheaterError::Other(err.to_string()))?,

            // TODO: remove events below
            Event::CreateAccountRequested((address, account_bytes)) => {
                // I think we can get rid of this, as we now add accounts
                // when they are a receiver of a transaction
                self.handle_create_account_requested(address.clone(), account_bytes)?;
            },
            _ => {},
        }

        Ok(ActorState::Running)
    }
}
