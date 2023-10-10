use block::{
    header::BlockHeader, Block, BlockHash, Certificate, ConvergenceBlock, GenesisBlock,
    ProposalBlock,
};
use events::{AccountBytes, AssignedQuorumMembership, Event, PeerData, Vote};
use miner::conflict_resolver::Resolver;
use primitives::{Address, NodeId, NodeType, PublicKey, QuorumId, QuorumKind, Signature};
use signer::engine::{QuorumData, QuorumMembers as InaugaratedMembers};
use std::{collections::HashMap, fmt::format};
use storage::vrrbdb::ApplyBlockResult;

use crate::{
    node_runtime::NodeRuntime,
    result::{NodeError, Result},
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
        // TODO: append blocks to only one instance of the DAG
        self.state_driver
            .dag
            .append_genesis(&block)
            .map_err(|err| {
                NodeError::Other(format!("Failed to append genesis block to DAG: {err:?}"))
            })?;

        let apply_result = self.state_driver.apply_block(Block::Genesis { block })?;

        Ok(apply_result)
    }

    // TODO:
    // check if from valid harvester
    // whoever sent the proposal block must be a valid harvester
    // sig_engine.quorum_members().is_harvester()
    fn handle_proposal_block_received(&mut self, block: ProposalBlock) -> Result<ApplyBlockResult> {
        if let Err(e) = self
            .state_driver
            .dag
            .append_proposal(&block, self.consensus_driver.sig_engine.clone())
        {
            let err_note = format!("Failed to append proposal block to DAG: {e:?}");
            return Err(NodeError::Other(err_note));
        }
        todo!()
    }

    /// Certifies and stores a convergence block within a node's state if certification succeeds
    // TODO: check if harvester, request a pre check
    // ConvergenceBlockPrecheckRequested
    fn handle_convergence_block_received(
        &mut self,
        mut block: ConvergenceBlock,
    ) -> Result<ApplyBlockResult> {
        self.consensus_driver.is_harvester()?;
        let apply_result = self
            .state_driver
            .append_convergence(&mut block)
            .map_err(|err| {
                NodeError::Other(format!(
                    "Could not append convergence block to DAG: {err:?}"
                ))
            })?;

        Ok(apply_result)
    }

    pub async fn handle_harvester_signature_received(
        &mut self,
        block_hash: String,
        node_id: NodeId,
        sig: Signature,
    ) -> Result<Certificate> {
        self.consensus_driver
            .sig_engine
            .verify(&node_id, &sig, &block_hash)
            .map_err(|err| NodeError::Other(err.to_string()))?;
        let set = self
            .state_driver
            .dag
            .add_signer_to_convergence_block(
                block_hash.clone(),
                sig,
                node_id,
                &self.consensus_driver.sig_engine,
            )
            .map_err(|err| NodeError::Other(err.to_string()))?;
        if set.len()
            < self
                .consensus_driver
                .sig_engine
                .quorum_members()
                .get_harvester_threshold()
        {
            return Err(NodeError::Other(format!("threshold not reached yet")));
        }

        let sig_set = set.into_iter().collect();
        let cert = self
            .form_convergence_certificate(block_hash, sig_set)
            .map_err(|err| NodeError::Other(err.to_string()))?;

        self.events_tx
            .send(Event::BlockCertificateCreated(cert.clone()).into())
            .await
            .map_err(|err| NodeError::Other(err.to_string()))?;
        Ok(cert)
    }

    pub fn form_convergence_certificate(
        &mut self,
        block_hash: String,
        sigs: Vec<(NodeId, Signature)>,
    ) -> Result<Certificate> {
        // TODO: figure out how to get next_root_hash back into cert
        // this should probably be part of the signature process
        self.consensus_driver.is_harvester()?;
        self.consensus_driver
            .sig_engine
            .verify_batch(&sigs, &block_hash)
            .map_err(|err| NodeError::Other(err.to_string()))?;
        if let Some(ref mut block) = self
            .state_driver
            .dag
            .get_pending_convergence_block_mut(&block_hash)
        {
            let root_hash = block.header.txn_hash.clone();
            let block_hash = block.hash.clone();
            let inauguration = if let Some(quorum) = &self.pending_quorum {
                Some(quorum.clone())
            } else {
                None
            };
            let cert = Certificate {
                signatures: sigs,
                //TODO: handle inauguration blocks
                inauguration: inauguration.clone(),
                root_hash,
                block_hash: block_hash.clone(),
            };
            //            if let Some(quorum_members) = inauguration {
            //                self.consensus_driver.sig_engine.set_quorum_members(
            //                    quorum_members
            //                        .0
            //                        .into_iter()
            //                        .map(|(_, data)| {
            //                            (data.quorum_kind, data.members.clone().into_iter().collect())
            //                        })
            //                        .collect(),
            //                );
            //                self.pending_quorum = None;
            //            }
            Ok(cert)
        } else {
            Err(NodeError::Other(format!(
                "unable to find convergence block: {} in pending convergence blocks in dag",
                block_hash.clone()
            )))
        }
    }

    // harvester sign and create cert
    pub async fn handle_block_certificate_created(
        &mut self,
        certificate: Certificate,
    ) -> Result<ConvergenceBlock> {
        // This is for when the local node is a harvester and forms the certificate
        self.handle_block_certificate_received(certificate).await
    }

    pub async fn handle_quorum_formed(&mut self) -> Result<()> {
        // This is probably where we want to put the logic for
        // taking a pending quorum and applying it
        todo!();
    }

    // recieve cert from network
    pub async fn handle_block_certificate_received(
        &mut self,
        certificate: Certificate,
    ) -> Result<ConvergenceBlock> {
        // This is for when a certificate is received from the network.
        self.verify_certificate(&certificate)?;
        let block = self
            .append_certificate_to_convergence_block(&certificate)?
            .ok_or(NodeError::Other(
                "certificate not appended to convergence block".to_string(),
            ))?;
        

        Ok(block.clone())
    }

    pub fn verify_certificate(&mut self, certificate: &Certificate) -> Result<()> {
        let cert_sigs = certificate.signatures.clone();
        if cert_sigs.len()
            < self
                .consensus_driver
                .sig_engine
                .quorum_members()
                .get_harvester_threshold()
        {
            return Err(NodeError::Other("threshold not reached".to_string()));
        }
        self.consensus_driver
            .sig_engine
            .verify_batch(&certificate.signatures, &certificate.block_hash)
            .map_err(|err| NodeError::Other(err.to_string()))?;

        Ok(())
    }

    pub fn append_certificate_to_convergence_block(
        &mut self,
        certificate: &Certificate,
    ) -> Result<Option<ConvergenceBlock>> {
        self.state_driver
            .append_certificate_to_convergence_block(&certificate)
            .map_err(|err| NodeError::Other(format!("{:?}", err)))
    }

    pub async fn handle_build_proposal_block_requested(
        &mut self,
        block: ConvergenceBlock,
    ) -> Result<ProposalBlock> {
        let block_hash = block.hash.clone();
        let block_round = block.header.round + 1;
        let block_epoch = block.header.epoch;
        let from = self.claim.clone();
        let sig_engine = self.consensus_driver.sig_engine.clone();
        let claims_map = self.consensus_driver.quorum_certified_claims.clone();
        self.mine_proposal_block(
            block_hash,
            claims_map,
            block_round,
            block_epoch,
            from,
            sig_engine,
        )
    }

    pub async fn handle_vote_received(&mut self, vote: Vote) -> Result<()> {
        self.consensus_driver.handle_vote_received(vote).await
    }

    pub async fn handle_node_added_to_peer_list(
        &mut self,
        peer_data: PeerData,
    ) -> Result<Option<HashMap<NodeId, AssignedQuorumMembership>>> {
        self.consensus_driver
            .handle_node_added_to_peer_list(peer_data)
            .await
    }

    pub fn handle_quorum_membership_assigment_created(
        &mut self,
        assigned_membership: AssignedQuorumMembership,
    ) -> Result<()> {
        self.consensus_driver
            .handle_quorum_membership_assigment_created(assigned_membership)
    }

    pub fn handle_quorum_membership_assigments_created(
        &mut self,
        assigned_membership: Vec<AssignedQuorumMembership>,
    ) -> Result<()> {
        self.consensus_driver
            .handle_quorum_membership_assigments_created(
                assigned_membership,
                self.config.id.clone(),
            )
    }

    pub async fn handle_convergence_block_precheck_requested<
        R: Resolver<Proposal = ProposalBlock>,
    >(
        &mut self,
        block: ConvergenceBlock,
        last_confirmed_block_header: BlockHeader,
        resolver: R,
    ) -> Result<()> {
        self.consensus_driver.is_harvester()?;
        match self.consensus_driver.precheck_convergence_block(
            block.clone(),
            last_confirmed_block_header,
            resolver,
            self.state_driver.dag.dag(),
        ) {
            Ok((true, true)) => {
                self.events_tx
                    .send(Event::SignConvergenceBlock(block.clone()).into())
                    .await
                    .map_err(|err| NodeError::Other(err.to_string()))?;
                Ok(())
            },
            Err(err) => return Err(NodeError::Other(err.to_string())),
            _ => {
                return Err(NodeError::Other(
                    "convergence block is not valid".to_string(),
                ))
            },
        }
    }

    pub async fn handle_sign_convergence_block(
        &mut self,
        block: ConvergenceBlock,
    ) -> Result<Signature> {
        self.consensus_driver.is_harvester()?;
        self.consensus_driver
            .sig_engine
            .sign(&block.hash)
            .map_err(|err| {
                NodeError::Other(format!(
                    "could not generate partial_signature on block: {}. err: {}",
                    block.hash.clone(),
                    err
                ))
            })
    }

    // TODO: Replace claims HashMap with claim_store_read_handle_factory
    pub fn handle_quorum_election_started(&mut self, header: BlockHeader) -> Result<()> {
        let claims = self.state_driver.read_handle().claim_store_values();
        let quorums = self
            .consensus_driver
            .handle_quorum_election_started(header, claims)?;

        let quorum_assignment: Vec<(QuorumKind, Vec<(NodeId, PublicKey)>)> = {
            quorums
                .clone()
                .iter()
                .filter_map(|quorum| {
                    quorum
                        .quorum_kind
                        .clone()
                        .map(|qk| (qk.clone(), quorum.members.clone()))
                })
                .collect()
        };

        let mut inaug_members = InaugaratedMembers(HashMap::new());

        quorum_assignment.iter().for_each(|quorum| {
            let quorum_id = QuorumId::new(quorum.0.clone(), quorum.1.clone());
            let quorum_data = QuorumData {
                id: quorum_id.clone(),
                quorum_kind: quorum.0.clone(),
                members: quorum.1.clone().into_iter().collect(),
            };
            inaug_members.0.insert(quorum_id, quorum_data);
        });
        self.pending_quorum = Some(inaug_members);

        //        let local_id = self.config.id.clone();
        //        for (qk, members) in quorum_assignment.iter() {
        //            if members
        //                .clone()
        //                .iter()
        //                .any(|(node_id, _)| node_id == &local_id)
        //            {
        //                self.consensus_driver.quorum_membership =
        //                    Some(QuorumId::new(qk.clone(), members.clone()));
        //                self.consensus_driver.quorum_kind = Some(qk.clone());
        //            }
        //        }
        Ok(())
    }

    pub fn handle_create_account_requested(
        &mut self,
        address: Address,
        account_bytes: AccountBytes,
    ) -> Result<()> {
        let account = bincode::deserialize(&account_bytes).map_err(|err| {
            NodeError::Other(format!("unable to deserialize account bytes: {err}"))
        })?;

        self.state_driver.insert_account(address, account)
    }
}
