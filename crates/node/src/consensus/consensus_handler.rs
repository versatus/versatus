use std::collections::{BTreeMap, HashSet};

use async_trait::async_trait;
use dkg_engine::dkg::DkgGenerator;
use events::{Event, EventMessage, EventPublisher, EventSubscriber, Vote};
use primitives::{NodeId, NodeType, ValidatorPublicKey};
use telemetry::info;
use theater::{Actor, ActorId, ActorImpl, ActorLabel, ActorState, Handler, TheaterError};
use vrrb_config::{QuorumMember, QuorumMembershipConfig};

use crate::{consensus::ConsensusModule, state_reader::StateReader};

#[async_trait]
impl<S: StateReader + Send + Sync + Clone> Handler<EventMessage> for ConsensusModule<S> {
    fn id(&self) -> ActorId {
        self.id.clone()
    }

    fn label(&self) -> ActorLabel {
        format!("Consensus::{}", self.id())
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
                if let Some(quorum_config) = self.quorum_driver.bootstrap_quorum_config.clone() {
                    let node_id = peer_data.node_id.clone();

                    let quorum_member_ids = quorum_config
                        .membership_config
                        .quorum_members
                        .iter()
                        .cloned()
                        .map(|member| member.node_id)
                        .collect::<Vec<NodeId>>();

                    self.dkg_engine
                        .add_peer_public_key(node_id.clone(), peer_data.validator_public_key);

                    if quorum_member_ids.contains(&node_id) {
                        self.quorum_driver
                            .bootstrap_quorum_available_nodes
                            .insert(node_id, (peer_data, true));
                    }

                    let available_nodes =
                        self.quorum_driver.bootstrap_quorum_available_nodes.clone();

                    let all_nodes_available =
                        available_nodes.iter().all(|(_, (_, is_online))| *is_online);

                    if all_nodes_available {
                        info!("All quorum members are online. Triggering genesis quorum elections");

                        if matches!(
                            self.quorum_driver.node_config.node_type,
                            primitives::NodeType::Bootstrap
                        ) {
                            self.quorum_driver
                                .assign_peer_list_to_quorums(available_nodes)
                                .await
                                .map_err(|err| TheaterError::Other(err.to_string()))?;
                        }
                    }
                }
            },
            Event::QuorumMembershipAssigmentCreated(assigned_membership) => {
                self.handle_quorum_membership_assigment_created(assigned_membership);

                let (part, node_id) = self
                    .generate_partial_commitment_message()
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                self.events_tx
                    .send(Event::PartCommitmentCreated(node_id, part).into())
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },
            Event::PartCommitmentCreated(node_id, part) => {
                self.handle_part_commitment_created(node_id, part);
            },
            Event::PartCommitmentAcknowledged(node_id) => {
                self.handle_part_commitment_acknowledged(node_id)?;
            },
            Event::QuorumElectionStarted(header) => {
                self.handle_quorum_election_started(header);
            },
            Event::MinerElectionStarted(header) => {
                self.handle_miner_election_started(header);
            },
            Event::Stop => {
                return Ok(ActorState::Stopped);
            },
            // This certifies txns once vote threshold is reached.
            Event::TransactionCertificateCreated { .. } => {
                // TODO: forward arguments
                self.handle_transaction_certificate_created();
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
                self.handle_convergence_block_partial_signature_created(
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
                self.handle_convergence_block_peer_signature_request(
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
                self.handle_convergence_block_precheck_requested(convergence_block, block_header);
            },
            Event::TxnsReadyForProcessing(txns) => {
                // Receives a batch of transactions from mempool and sends
                // them to scheduler to get it validated and voted
                self.handle_txns_ready_for_processing(txns);
            },

            // Receive votes from scheduler
            Event::TxnsValidated {
                votes,
                quorum_threshold,
            } => {
                for vote in votes.iter().flatten() {
                    self.validate_vote(vote.clone(), quorum_threshold);
                }
            },
            _ => {},
        }

        Ok(ActorState::Running)
    }
}
