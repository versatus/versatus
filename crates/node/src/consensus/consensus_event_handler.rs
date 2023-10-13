use super::ConsensusModule;
use crate::{NodeError, Result};
use block::{header::BlockHeader, Block, ConvergenceBlock, InnerBlock, ProposalBlock};
use bulldag::graph::BullDag;
use ethereum_types::U256;
use events::{AssignedQuorumMembership, PeerData};
use miner::conflict_resolver::Resolver;
use primitives::{NodeId, NodeType, PublicKey};
use quorum::quorum::Quorum;
use ritelinked::{LinkedHashMap, LinkedHashSet};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::{Arc, RwLock},
};
use vrrb_config::QuorumMember;
use vrrb_config::QuorumMembershipConfig;
use vrrb_core::claim::Claim;
use vrrb_core::transactions::TransactionDigest;

impl ConsensusModule {
    pub async fn handle_node_added_to_peer_list(
        &mut self,
        peer_data: PeerData,
    ) -> Result<Option<HashMap<NodeId, AssignedQuorumMembership>>> {
        if let Some(quorum_config) = self.quorum_driver.bootstrap_quorum_config.clone() {
            let node_id = peer_data.node_id.clone();

            let quorum_member_ids = quorum_config
                .membership_config
                .quorum_members
                .iter()
                .map(|(_, member)| member.node_id.to_owned())
                .collect::<Vec<NodeId>>();

            if quorum_member_ids.contains(&node_id) {
                self.quorum_driver
                    .bootstrap_quorum_available_nodes
                    .insert(node_id, (peer_data.clone(), true));
            }

            let available_nodes = self.quorum_driver.bootstrap_quorum_available_nodes.clone();

            let all_nodes_available = available_nodes.iter().all(|(_, (_, is_online))| *is_online);

            if all_nodes_available {
                telemetry::info!(
                    "All quorum members are online. Triggering genesis quorum elections"
                );

                if matches!(
                    self.quorum_driver.node_config.node_type,
                    primitives::NodeType::Bootstrap
                ) {
                    let assignments = self
                        .quorum_driver
                        .assign_peer_list_to_quorums(available_nodes)
                        .await?;

                    return Ok(Some(assignments));
                }
            }
        }

        Ok(None)
    }

    pub fn handle_quorum_membership_assigment_created(
        &mut self,
        assigned_membership: AssignedQuorumMembership,
    ) -> Result<()> {
        if matches!(self.node_config.node_type, NodeType::Bootstrap) {
            return Err(NodeError::Other(format!(
                "bootstrap node {} cannot belong to a quorum",
                &self.node_config.id
            )));
        }

        if let Some(membership_config) = &self.quorum_driver.membership_config {
            telemetry::info!(
                "{} already belongs to a {} quorum",
                &self.node_config.id,
                membership_config.quorum_kind
            );
            return Err(NodeError::Other(format!(
                "{} already belongs to a {} quorum",
                &self.node_config.id, membership_config.quorum_kind
            )));
        }

        let quorum_kind = assigned_membership.quorum_kind.clone();
        let quorum_membership_config = QuorumMembershipConfig {
            quorum_members: assigned_membership
                .peers
                .into_iter()
                .map(|peer| {
                    (
                        peer.node_id.clone(),
                        QuorumMember {
                            node_id: peer.node_id,
                            kademlia_peer_id: peer.kademlia_peer_id,
                            // TODO: get from kademlia metadata
                            node_type: NodeType::Validator,
                            udp_gossip_address: peer.udp_gossip_addr,
                            raptorq_gossip_address: peer.raptorq_gossip_addr,
                            kademlia_liveness_address: peer.kademlia_liveness_addr,
                            validator_public_key: peer.validator_public_key,
                        },
                    )
                })
                .collect(),
            quorum_kind,
        };

        self.quorum_driver.membership_config = Some(quorum_membership_config);
        self.quorum_kind = Some(assigned_membership.quorum_kind);
        Ok(())
    }

    pub fn handle_quorum_membership_assigments_created(
        &mut self,
        assigned_memberships: Vec<AssignedQuorumMembership>,
        local_node_id: NodeId,
    ) -> Result<()> {
        if matches!(self.node_config.node_type, NodeType::Bootstrap) {
            dbg!("node is boostrap, aborting");
            return Err(NodeError::Other(format!(
                "bootstrap node {} cannot belong to a quorum",
                &self.node_config.id
            )));
        }

        let mut local_membership = assigned_memberships.clone();
        local_membership.retain(|membership| membership.node_id == local_node_id);
        if let Some(membership) = local_membership.pop() {
            let quorum_kind = membership.quorum_kind.clone();
            let config = QuorumMembershipConfig {
                quorum_members: membership
                    .peers
                    .into_iter()
                    .map(|peer| {
                        (
                            peer.node_id.clone(),
                            QuorumMember {
                                node_id: peer.node_id,
                                kademlia_peer_id: peer.kademlia_peer_id,
                                // TODO: get from kademlia metadata
                                node_type: NodeType::Validator,
                                udp_gossip_address: peer.udp_gossip_addr,
                                raptorq_gossip_address: peer.raptorq_gossip_addr,
                                kademlia_liveness_address: peer.kademlia_liveness_addr,
                                validator_public_key: peer.validator_public_key,
                            },
                        )
                    })
                    .collect(),
                quorum_kind,
            };
            self.quorum_driver.membership_config = Some(config.clone());
            self.quorum_kind = Some(membership.quorum_kind);
        }

        let mut unique_quorums = HashSet::new();
        for mem in assigned_memberships.iter() {
            let kind = mem.quorum_kind.clone();
            let mut peers = mem
                .peers
                .clone()
                .into_iter()
                .map(|peer| (peer.node_id.clone(), peer.validator_public_key.clone()))
                .collect::<HashSet<(NodeId, PublicKey)>>();

            peers.insert((mem.node_id.clone(), mem.pub_key.clone()));
            let mut peers = peers.into_iter().collect::<Vec<_>>();
            peers.sort();

            unique_quorums.insert((kind, peers));
        }

        let quorums = unique_quorums.into_iter().collect::<Vec<_>>();
        self.sig_engine.set_quorum_members(quorums);
        Ok(())
    }

    pub fn handle_quorum_election_started(
        &mut self,
        header: BlockHeader,
        claims: HashMap<NodeId, Claim>,
    ) -> Result<Vec<Quorum>> {
        let quorum = self
            .quorum_driver
            .elect_quorums(claims, header)
            .map_err(|err| NodeError::Other(format!("failed to elect quorum: {err}")))?;

        Ok(quorum)
    }

    pub fn handle_miner_election_started(
        &mut self,
        header: BlockHeader,
        claims: HashMap<String, Claim>,
    ) -> Result<BTreeMap<U256, Claim>> {
        let election_results: BTreeMap<U256, Claim> =
            self.quorum_driver.elect_miner(claims, header.block_seed);
        self.miner_election_results = Some(election_results.clone());
        Ok(election_results)
    }

    fn precheck_convergence_block_get_proposal_blocks(
        &mut self,
        block_hash: String,
        proposal_block_hashes: Vec<String>,
        dag: Arc<RwLock<BullDag<Block, String>>>,
    ) -> Result<Vec<ProposalBlock>> {
        let proposals = {
            if let Ok(dag) = dag.read() {
                let proposals: Vec<ProposalBlock> = proposal_block_hashes
                    .iter()
                    .filter_map(|hash| {
                        dag.get_vertex(hash.clone())
                            .and_then(|vtx| match vtx.get_data() {
                                Block::Proposal { block } => Some(block.clone()),
                                _ => None,
                            })
                    })
                    .collect();
                if proposals.len() != proposal_block_hashes.len() {
                    return Err(NodeError::Other(format!(
                        "missing proposal blocks referenced by convergence block: {}",
                        block_hash.clone()
                    )));
                }
                Ok(proposals)
            } else {
                return Err(NodeError::Other(
                    "could not acquire read lock on dag".to_string(),
                ));
            }
        };

        proposals
    }

    fn precheck_resolve_proposal_block_conflicts<R: Resolver<Proposal = ProposalBlock>>(
        &mut self,
        block: ConvergenceBlock,
        proposals: Vec<ProposalBlock>,
        resolver: R,
    ) -> LinkedHashMap<String, LinkedHashSet<TransactionDigest>> {
        let resolved: LinkedHashMap<String, LinkedHashSet<TransactionDigest>> = resolver
            .resolve(
                &proposals,
                block.get_header().round,
                block.get_header().block_seed,
            )
            .iter()
            .map(|block| (block.hash.clone(), block.txn_id_set()))
            .collect();

        resolved
    }

    fn precheck_resolved_transactions_are_valid(
        &mut self,
        block: ConvergenceBlock,
        resolved: LinkedHashMap<String, LinkedHashSet<TransactionDigest>>,
    ) -> bool {
        let mut valid_txns = true;
        let comp: Vec<bool> = resolved
            .iter()
            .filter_map(
                |(pblock_hash, txn_id_set)| match block.txns.get(pblock_hash) {
                    Some(set) => Some(set == txn_id_set),
                    None => None,
                },
            )
            .collect();

        if comp.len() != block.txns.len() {
            valid_txns = false;
        }

        if comp.iter().any(|&value| !value) {
            valid_txns = false;
        }

        valid_txns
    }

    fn precheck_get_proposal_block_claims(
        &mut self,
        proposals: Vec<ProposalBlock>,
    ) -> LinkedHashMap<String, LinkedHashSet<U256>> {
        proposals
            .iter()
            .map(|block| {
                let block_claims = block
                    .claims
                    .keys()
                    .into_iter()
                    .map(|key| key.clone())
                    .collect();
                (block.hash.clone(), block_claims)
            })
            .collect()
    }

    fn precheck_convergence_block_transactions<R: Resolver<Proposal = ProposalBlock>>(
        &mut self,
        block: ConvergenceBlock,
        proposal_block_hashes: Vec<String>,
        resolver: R,
        dag: Arc<RwLock<BullDag<Block, String>>>,
    ) -> Result<(bool, bool)> {
        let proposals = self.precheck_convergence_block_get_proposal_blocks(
            block.hash.clone(),
            proposal_block_hashes,
            dag.clone(),
        )?;

        let resolved = self.precheck_resolve_proposal_block_conflicts(
            block.clone(),
            proposals.clone(),
            resolver,
        );

        let proposal_claims = self.precheck_get_proposal_block_claims(proposals);

        Ok((
            self.precheck_resolved_transactions_are_valid(block.clone(), resolved),
            self.precheck_convergence_block_claims(block.claims.clone(), proposal_claims),
        ))
    }

    fn precheck_convergence_block_claims(
        &mut self,
        convergence_claims: LinkedHashMap<String, LinkedHashSet<U256>>,
        proposal_claims: LinkedHashMap<String, LinkedHashSet<U256>>,
    ) -> bool {
        let mut valid_claims = true;
        let comp: Vec<bool> = proposal_claims
            .iter()
            .filter_map(
                |(pblock_hash, claim_hash_set)| match convergence_claims.get(pblock_hash) {
                    Some(set) => Some(set == claim_hash_set),
                    None => None,
                },
            )
            .collect();

        if comp.len() != convergence_claims.len() {
            valid_claims = false;
        }

        if comp.iter().any(|&value| !value) {
            valid_claims = false;
        }

        valid_claims
    }

    pub fn precheck_convergence_block<R: Resolver<Proposal = ProposalBlock>>(
        &mut self,
        block: ConvergenceBlock,
        // TODO: use last_confirmed_block_header for seed & round
        // for conflict resolution
        _last_confirmed_block_header: BlockHeader,
        resolver: R,
        dag: Arc<RwLock<BullDag<Block, String>>>,
    ) -> Result<(bool, bool)> {
        self.is_harvester()?;
        self.precheck_convergence_block_miner_is_winner(block.clone())?;
        let proposal_block_hashes = block.header.ref_hashes.clone();
        self.precheck_convergence_block_transactions(block, proposal_block_hashes, resolver, dag)
    }

    pub fn precheck_convergence_block_miner_is_winner(&self, block: ConvergenceBlock) -> Result<()> {
        let miner = block.header.miner_claim.clone();

        if let Some(results) = &self.miner_election_results {
            for (idx, claim) in results.clone().values().enumerate() {
                if idx < 5 {
                    if claim == &miner {
                        return Ok(())
                    }
                } else {
                    break
                }
            }
        }
        return Err(NodeError::Other("miner was not elected".to_string()));
    }
}
