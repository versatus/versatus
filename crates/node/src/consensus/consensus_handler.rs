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
                self.handle_node_added_to_peer_list(peer_data.clone())
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                self.add_peer_public_key_to_dkg_state(
                    peer_data.node_id.clone(),
                    peer_data.validator_public_key,
                );
            },
            Event::QuorumMembershipAssigmentCreated(assigned_membership) => {
                self.handle_quorum_membership_assigment_created(assigned_membership);

                let (part, node_id) = self
                    .generate_partial_commitment_message()
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                let event = Event::PartCommitmentCreated(node_id, part);

                let em = EventMessage::new(Some("network-events".into()), event);

                self.events_tx
                    .send(em)
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

            Event::TransactionCertificateRequested {
                votes,
                txn_id,
                quorum_key,
                farmer_id,
                txn,
                quorum_threshold,
            } => {
                self.handle_transaction_certificate_requested(
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
                // TODO: forward arguments
                self.handle_transaction_certificate_created(
                    votes,
                    signature,
                    digest,
                    execution_result,
                    farmer_id,
                    txn,
                    is_valid,
                );
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
