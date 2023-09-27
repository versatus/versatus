use std::{
    collections::HashMap,
    fmt::format,
    hash::Hash,
    sync::{Arc, RwLock},
};

use block::{
    header::BlockHeader, vesting::GenesisConfig, Block, Certificate, ClaimHash, ConvergenceBlock,
    GenesisBlock, ProposalBlock, QuorumCertifiedTxnList, RefHash,
};
use bulldag::graph::BullDag;
use dkg_engine::prelude::{DkgEngine, DkgEngineConfig, ReceiverId, SenderId};
use ethereum_types::U256;
use events::{AssignedQuorumMembership, EventPublisher, PeerData};
use hbbft::sync_key_gen::{Ack, Part};
use mempool::{LeftRightMempool, MempoolReadHandleFactory, TxnRecord};
use miner::{Miner, MinerConfig};
use primitives::{
    Address, Epoch, NodeId, NodeType, PublicKey, QuorumKind, Round, ValidatorPublicKey,
};
use quorum::quorum::Quorum;
use ritelinked::LinkedHashMap;
use secp256k1::Message;
use storage::vrrbdb::{ApplyBlockResult, VrrbDbConfig, VrrbDbReadHandle};
use theater::{ActorId, ActorState};
use tokio::task::JoinHandle;
use utils::payload::digest_data_to_bytes;
use vrrb_config::{NodeConfig, QuorumMembershipConfig};
use vrrb_core::{
    account::{Account, UpdateArgs},
    claim::Claim,
    transactions::{
        NewTransferArgs, Token, Transaction, TransactionDigest, TransactionKind, Transfer,
    },
};

use crate::{
    consensus::{ConsensusModule, ConsensusModuleConfig},
    mining_module::{MiningModule, MiningModuleConfig},
    node_runtime::NodeRuntime,
    result::{NodeError, Result},
    state_manager::{DagModule, StateManager, StateManagerConfig},
};

pub const PULL_TXN_BATCH_SIZE: usize = 100;

impl NodeRuntime {
    pub fn handle_block_received(&mut self, block: Block) -> Result<ApplyBlockResult> {
        match block {
            Block::Genesis { block } => self.handle_genesis_block_received(block),
            Block::Proposal { block } => self.handle_proposal_block_received(block),
            Block::Convergence { block } => self.handle_convergence_block_received(block),
        }
    }

    fn handle_genesis_block_received(&mut self, block: GenesisBlock) -> Result<ApplyBlockResult> {
        //
        //
        // TODO: append blocks to only one instance of the DAG
        //
        self.dag_driver.append_genesis(&block).map_err(|err| {
            NodeError::Other(format!("Failed to append genesis block to DAG: {err:?}"))
        })?;

        let apply_result = self.state_driver.apply_block(Block::Genesis { block })?;

        Ok(apply_result)
    }

    fn handle_proposal_block_received(&mut self, block: ProposalBlock) -> Result<ApplyBlockResult> {
        if let Err(e) = self.state_driver.dag.append_proposal(&block) {
            let err_note = format!("Failed to append proposal block to DAG: {e:?}");
            return Err(NodeError::Other(err_note));
        }
        todo!()
    }

    /// Certifies and stores a convergence block within a node's state if certification succeeds
    fn handle_convergence_block_received(
        &mut self,
        block: ConvergenceBlock,
    ) -> Result<ApplyBlockResult> {
        self.has_required_node_type(NodeType::Validator, "certify convergence block")?;
        self.belongs_to_correct_quorum(QuorumKind::Harvester, "certify convergence block")?;

        self.state_driver
            .dag
            .append_convergence(&block)
            .map_err(|err| {
                NodeError::Other(format!(
                    "Could not append convergence block to DAG: {err:?}"
                ))
            })?;

        if block.certificate.is_none() {
            if let Some(_) = self.dag_driver.last_confirmed_block_header() {
                let certificate = self.certify_convergence_block(block.clone());
            }
        }

        let apply_result = self
            .state_driver
            .apply_block(Block::Convergence { block })?;

        Ok(apply_result)
    }

    pub fn handle_block_certificate_created(&mut self, certificate: Certificate) -> Result<()> {
        //
        //         let mut mine_block: Option<ConvergenceBlock> = None;
        //         let block_hash = certificate.block_hash.clone();
        //         if let Ok(Some(Block::Convergence { mut block })) =
        //             self.dag.write().map(|mut bull_dag| {
        //                 bull_dag
        //                     .get_vertex_mut(block_hash)
        //                     .map(|vertex| vertex.get_data())
        //             })
        //         {
        //             block.append_certificate(certificate.clone());
        //             self.last_confirmed_block_header = Some(block.get_header());
        //             mine_block = Some(block.clone());
        //         }
        //         if let Some(block) = mine_block {
        //             let proposal_block = Event::MineProposalBlock(
        //                 block.hash.clone(),
        //                 block.get_header().round,
        //                 block.get_header().epoch,
        //                 self.claim.clone(),
        //             );
        //             if let Err(err) = self
        //                 .events_tx
        //                 .send(EventMessage::new(None, proposal_block.clone()))
        //                 .await
        //             {
        //                 let err_msg = format!(
        //                     "Error occurred while broadcasting event {proposal_block:?}: {err:?}"
        //                 );
        //                 return Err(TheaterError::Other(err_msg));
        //             }
        //         } else {
        //             telemetry::debug!("Missing ConvergenceBlock for certificate: {certificate:?}");
        //         }
        //
        todo!()
    }

    pub async fn handle_node_added_to_peer_list(
        &mut self,
        peer_data: PeerData,
    ) -> Result<Option<HashMap<NodeId, AssignedQuorumMembership>>> {
        self.consensus_driver
            .handle_node_added_to_peer_list(peer_data)
            .await
    }

    pub fn handle_proposal_block_mine_request_created(
        &mut self,
        ref_hash: RefHash,
        round: Round,
        epoch: Epoch,
        claim: Claim,
    ) -> Result<ProposalBlock> {
        self.has_required_node_type(NodeType::Validator, "create proposal block")?;
        self.belongs_to_correct_quorum(QuorumKind::Harvester, "create proposal block")?;

        // let proposal_block = self
        //     .consensus_driver
        //     .handle_proposal_block_mine_request_created(
        //         args.ref_hash,
        //         args.round,
        //         args.epoch,
        //         args.claim,
        //     )?;
        //
        // Ok(proposal_block)
        todo!()
    }

    pub fn handle_part_commitment_created(
        &mut self,
        sender_id: SenderId,
        part: Part,
    ) -> Result<(ReceiverId, SenderId, Ack)> {
        self.consensus_driver
            .handle_part_commitment_created(sender_id, part)
    }

    pub fn handle_part_commitment_acknowledged(
        &mut self,
        receiver_id: ReceiverId,
        sender_id: SenderId,
        ack: Ack,
    ) -> Result<()> {
        self.consensus_driver
            .handle_part_commitment_acknowledged(receiver_id, sender_id, ack)
    }
    pub fn handle_all_ack_messages(&mut self) -> Result<()> {
        self.consensus_driver.handle_all_ack_messages()
    }

    pub fn handle_quorum_membership_assigment_created(
        &mut self,
        assigned_membership: AssignedQuorumMembership,
    ) -> Result<()> {
        self.consensus_driver
            .handle_quorum_membership_assigment_created(assigned_membership)
    }
    pub fn handle_convergence_block_precheck_requested(
        &mut self,
        block: ConvergenceBlock,
        last_confirmed_block_header: BlockHeader,
    ) {
        self.consensus_driver
            .precheck_convergence_block(block, last_confirmed_block_header);
    }

    pub fn handle_quorum_election_started(&mut self, header: BlockHeader) -> Result<Quorum> {
        let claims = self.state_driver.read_handle().claim_store_values();
        let quorum = self
            .consensus_driver
            .handle_quorum_election_started(header, claims)?;

        Ok(quorum)
    }

    pub fn handle_vote_received(&mut self) {
        //
    }
}
