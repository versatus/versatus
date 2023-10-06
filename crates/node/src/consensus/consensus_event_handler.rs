// use dkg_engine::{
//     dkg::DkgGenerator,
//     prelude::{ReceiverId, SenderId},
// };
use super::ConsensusModule;
use crate::{NodeError, Result};
use std::{collections::{BTreeMap, HashMap}, sync::{Arc, Mutex, RwLock}};

use block::{header::BlockHeader, BlockHash, ConvergenceBlock, Block, ProposalBlock, InnerBlock};
use bulldag::graph::BullDag;
//use dkg_engine::{
//    dkg::DkgGenerator,
//    prelude::{ReceiverId, SenderId},
//};
use ethereum_types::U256;
use events::{AssignedQuorumMembership, PeerData, Vote};
use hbbft::{
    crypto::PublicKeySet,
    sync_key_gen::{Ack, Part},
};
use maglev::Maglev;
use miner::{conflict_resolver::Resolver, block_builder::BlockBuilder};
use primitives::{
    FarmerQuorumThreshold, NodeId, NodeType, ProgramExecutionOutput, PublicKeyShareVec,
    RawSignature, TxnValidationStatus, ValidatorPublicKeyShare, Signature,
};
use quorum::quorum::Quorum;
use ritelinked::{LinkedHashMap, LinkedHashSet};
use signer::signer::SignatureProvider;
use vrrb_config::QuorumMember;
use vrrb_config::QuorumMembershipConfig;
use vrrb_core::claim::Claim;
use vrrb_core::transactions::{TransactionDigest, TransactionKind};

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

        // self.add_peer_public_key_to_dkg_state(
        //     peer_data.node_id.clone(),
        //     peer_data.validator_public_key,
        // );

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
        Ok(())
    }

    // pub fn handle_part_commitment_created(
    //     &mut self,
    //     sender_id: SenderId,
    //     part: Part,
    // ) -> Result<(ReceiverId, SenderId, Ack)> {
    //     if let Ok(membership_config) = self.membership_config_owned() {
    //         if sender_id != self.node_config.id
    //             && !membership_config.quorum_members.contains_key(&sender_id)
    //         {
    //             let msg = format!("Node {} is not a quorum member", self.node_config.id);

    //             return Err(NodeError::Other(msg));
    //         }
    //     }

    //     self.dkg_engine
    //         .dkg_state
    //         .part_message_store_mut()
    //         .entry(sender_id.clone())
    //         .or_insert_with(|| part);

    //     self.dkg_engine
    //         .ack_partial_commitment(sender_id)
    //         .map_err(|err| NodeError::Other(err.to_string()))
    // }

    // pub fn handle_part_commitment_acknowledged(
    //     &mut self,
    //     receiver_id: ReceiverId,
    //     sender_id: SenderId,
    //     ack: Ack,
    // ) -> Result<()> {
    //     self.dkg_engine
    //         .dkg_state
    //         .ack_message_store_mut()
    //         .entry((receiver_id, sender_id))
    //         .or_insert_with(|| ack);

    //     Ok(())
    // }

    // pub fn handle_all_ack_messages(&mut self) -> Result<()> {
    //     self.dkg_engine.handle_ack_messages()?;
    //     Ok(())
    // }

    // pub fn generate_keysets(&mut self) -> Result<Option<PublicKeySet>> {
    //     let res = self
    //         .dkg_engine
    //         .generate_key_sets()
    //         .map_err(|err| NodeError::Other(err.to_string()))?;
    //     self.sig_provider = SignatureProvider::from(&self.dkg_engine.dkg_state);
    //     Ok(res)
    // }

    pub fn handle_quorum_election_started(
        &mut self,
        header: BlockHeader,
        claims: HashMap<NodeId, Claim>,
    ) -> Result<Quorum> {
        let quorum = self
            .quorum_driver
            .elect_quorum(claims, header)
            .map_err(|err| NodeError::Other(format!("failed to elect quorum: {err}")))?;

        Ok(quorum)
    }

    pub fn handle_miner_election_started(
        &mut self,
        header: BlockHeader,
        claims: HashMap<String, Claim>,
    ) -> Result<(U256, Claim)> {
        let mut election_results: BTreeMap<U256, Claim> =
            self.quorum_driver.elect_miner(claims, header.block_seed);

        let winner = self.quorum_driver.get_winner(&mut election_results);

        Ok(winner)
    }

    // pub async fn handle_txns_ready_for_processing(
    //     &mut self,
    //     txns: Vec<TransactionKind>,
    // ) -> Result<()> {
    //     let keys: Vec<ByteSlice48Bit> = self
    //         .dkg_engine
    //         .dkg_state
    //         .peer_public_keys()
    //         .values()
    //         .map(|pk| pk.to_bytes())
    //         .collect();

    //     let maglev_hash_ring = Maglev::new(keys);
    //     // let mut new_txns = vec![];

    //     for txn in txns {
    //         // match maglev_hash_ring.get(&txn.id()).cloned() {
    //         //     Some(group_public_key) if group_public_key == self.group_public_key => {
    //         //         new_txns.push(txn)
    //         //     },
    //         //     Some(group_public_key) => {
    //         //         if let Some(broadcast_addresses) =
    //         //             self.neighbouring_farmer_quorum_peers.get(&group_public_key)
    //         //         {
    //         //             let addresses: Vec<SocketAddr> =
    //         //                 broadcast_addresses.iter().cloned().collect();
    //         //             self.broadcast_events_tx
    //         //                 .send(EventMessage::new(
    //         //                     None,
    //         //                     Event::ForwardTxn((txn.1.clone(), addresses.clone())),
    //         //                 ))
    //         //                 .await
    //         //                 .map_err(|err| {
    //         //                     theater::TheaterError::Other(format!(
    //         //                         "failed to forward txn {:?} to peers {:?}: {}",
    //         //                         txn.1, addresses, err
    //         //                     ))
    //         //                 })?;
    //         //         }
    //         //     },
    //         //     _ => new_txns.push(txn),
    //         // }
    //     }

    //     // if let Some(sig_provider) = self.sig_provider.clone() {
    //     //     if let Err(err) = self.sync_jobs_sender.send(Job::Farm((
    //     //         new_txns,
    //     //         self.farmer_id.clone(),
    //     //         self.farmer_node_idx,
    //     //         self.group_public_key.clone(),
    //     //         sig_provider,
    //     //         self.quorum_threshold,
    //     //     ))) {
    //     //         telemetry::error!("error sending job to scheduler: {}", err);
    //     //     }
    //     // }

    //     Ok(())
    // }

    pub fn handle_convergence_block_partial_signature_created(
        &mut self,
        block_hash: BlockHash,
        public_key_share: ValidatorPublicKeyShare,
        partial_signature: Signature,
    ) {
        //         if let Some(certificates_share) =
        //             self.convergence_block_certificates.get(&block_hash)
        //         {
        //             let mut new_certificate_share = certificates_share.clone();
        //             if let Ok(block_hash_bytes) = hex::decode(block_hash.clone()) {
        //                 if let Ok(signature) =
        //                     TryInto::<[u8; 96]>::try_into(partial_signature.clone())
        //                 {
        //                     if let Ok(signature_share) =
        // SignatureShare::from_bytes(signature) {                         if
        // public_key_share.verify(&signature_share, block_hash_bytes) {
        // new_certificate_share.insert((
        // self.harvester_id,                                 public_key_share,
        //                                 partial_signature.clone(),
        //                             ));
        //                             self.convergence_block_certificates.push(
        //                                 block_hash.clone(),
        //                                 new_certificate_share.clone(),
        //                             );
        //                             if let Some(sig_provider) = self.sig_provider.as_ref() {
        //                                 if new_certificate_share.len()
        //                                     <= sig_provider.quorum_config.upper_bound as
        // usize                                 {
        //                                     self
        //                                         .broadcast_events_tx
        //                                         .send(EventMessage::new(
        //                                             None,
        //                                             Event::SendPeerConvergenceBlockSign(
        //                                                 self.harvester_id,
        //                                                 block_hash.clone(),
        //                                                 public_key_share.to_bytes().to_vec(),
        //                                                 partial_signature,
        //                                             ),
        //                                         ))
        //                                         .await.map_err(|err|
        // theater::TheaterError::Other(
        // format!("failed to send peer convergence block sign: {err}")
        // ))?;
        //
        //                                     self.generate_and_broadcast_certificate(
        //                                         block_hash,
        //                                         &new_certificate_share,
        //                                         sig_provider,
        //                                     )
        //                                     .await?;
        //                                 }
        //                             }
        //                         }
        //                     }
        //                 }
        //             }
        //         }
    }

    fn precheck_convergence_block_get_proposal_blocks(
        &mut self,
        block_hash: String,
        proposal_block_hashes: Vec<String>,
        dag: Arc<RwLock<BullDag<Block, String>>>,
    ) -> Result<Vec<ProposalBlock>> {
        let proposals = {
            if let Ok(dag) = dag.read() {
                let proposals: Vec<ProposalBlock> = proposal_block_hashes.iter().filter_map(|hash| {
                    dag.get_vertex(hash.clone()).and_then(|vtx| {
                        match vtx.get_data() {
                            Block::Proposal { block } => Some(block.clone()),
                            _ => None
                        }
                    })
                }).collect();
                if proposals.len() != proposal_block_hashes.len() {
                    return Err(
                        NodeError::Other(
                            format!(
                                "missing proposal blocks referenced by convergence block: {}",
                                block_hash.clone()
                            )
                        )
                    )
                }
                Ok(proposals)
            } else {
                return Err(NodeError::Other("could not acquire read lock on dag".to_string()))
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
        let resolved: LinkedHashMap<String, LinkedHashSet<TransactionDigest>> = resolver.resolve(
            &proposals, 
            block.get_header().round, 
            block.get_header().block_seed
        ).iter().map(|block| {
            (block.hash.clone(), block.txn_id_set())
        }).collect();
                    
        resolved
    }

    fn precheck_resolved_transactions_are_valid(
        &mut self,
        block: ConvergenceBlock,
        resolved: LinkedHashMap<String, LinkedHashSet<TransactionDigest>>,
    ) -> bool {

        let mut valid_txns = true;
        let comp: Vec<bool> = resolved.iter().filter_map(|(pblock_hash, txn_id_set)| {
            match block.txns.get(pblock_hash) {
                Some(set) => { Some(set == txn_id_set) }
                None => { None }
            }
        }).collect();

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
        proposals.iter().map(|block| {
            let block_claims = block.claims
                .keys()
                .into_iter()
                .map(|key| key.clone())
                .collect();
            (block.hash.clone(), block_claims)
        }).collect()
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
            dag.clone()
        )?;
        
        let resolved = self.precheck_resolve_proposal_block_conflicts(
            block.clone(), 
            proposals.clone(), 
            resolver, 
        );
        
        let proposal_claims = self.precheck_get_proposal_block_claims(proposals);

        Ok((self.precheck_resolved_transactions_are_valid(block.clone(), resolved),
            self.precheck_convergence_block_claims(block.claims.clone(), proposal_claims)
        ))
    }

    fn precheck_convergence_block_claims(
        &mut self,
        convergence_claims: LinkedHashMap<String, LinkedHashSet<U256>>,
        proposal_claims: LinkedHashMap<String, LinkedHashSet<U256>>,
    ) -> bool {
        let mut valid_claims = true;
        let comp: Vec<bool> = proposal_claims.iter()
            .filter_map(|(pblock_hash, claim_hash_set)| {
                match convergence_claims.get(pblock_hash) {
                    Some(set) => Some(set == claim_hash_set),
                    None => None,
                }
            }).collect();

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
        dag: Arc<RwLock<BullDag<Block, String>>>
    ) -> Result<(bool, bool)> {

        let proposal_block_hashes = block.header.ref_hashes.clone();
        self.precheck_convergence_block_transactions(
            block, 
            proposal_block_hashes, 
            resolver, 
            dag
        ) 
    }

    pub fn handle_convergence_block_peer_signature_request(
        &mut self,
        node_id: NodeId,
        block_hash: BlockHash,
        public_key_share: PublicKeyShareVec,
        partial_signature: Signature,
    ) {
        //     let mut pb_key_share = None;
        //     let preliminary_check = TryInto::<[u8; 48]>::try_into(public_key_share_bytes)
        //         .and_then(|public_key_share_bytes| {
        //             PublicKeyShare::from_bytes(public_key_share_bytes).map_err(|e| {
        //                 format!("Invalid Public Key, Expected 48byte array:
        // {e}").into_bytes()             })
        //         })
        //         .and_then(|public_key_share| {
        //             pb_key_share = Some(public_key_share);
        //             TryInto::<[u8; 96]>::try_into(partial_signature.clone())
        //                 .and_then(|signature_share_bytes| {
        //                     SignatureShare::from_bytes(signature_share_bytes).map_err(|e| {
        //                         format!("Invalid Signature, Expected 96byte array: {e}")
        //                             .into_bytes()
        //                     })
        //                 })
        //                 .and_then(|signature_share| {
        //                     hex::decode(block_hash.clone())
        //                         .map_err(|e| {
        //                             format!(
        //                                 "Invalid Hex Representation of Signature Share: {e}",
        //                             )
        //                             .into_bytes()
        //                         })
        //                         .and_then(|block_hash_bytes| {
        //                             if public_key_share
        //                                 .verify(&signature_share, block_hash_bytes)
        //                             {
        //                                 Ok(())
        //                             } else {
        //                                 Err("signature verification failed"
        //                                     .to_string()
        //                                     .into_bytes())
        //                             }
        //                         })
        //                 })
        //         });
        //
        //     if preliminary_check.is_ok() {
        //         if let Some(certificates_share) =
        //             self.convergence_block_certificates.get(&block_hash)
        //         {
        //             let mut new_certificate_share = certificates_share.clone();
        //             if let Some(pb_key_share) = pb_key_share {
        //                 new_certificate_share.insert((
        //                     node_idx,
        //                     pb_key_share,
        //                     partial_signature,
        //                 ));
        //                 self.convergence_block_certificates
        //                     .push(block_hash.clone(), new_certificate_share.clone());
        //                 if let Some(sig_provider) = self.sig_provider.as_ref() {
        //                     self.generate_and_broadcast_certificate(
        //                         block_hash,
        //                         &new_certificate_share,
        //                         sig_provider,
        //                     )
        //                     .await?;
        //                 }
        //             }
        //         }
        //     }
        //
    }

    pub fn handle_transaction_certificate_requested(
        &mut self,
        votes: Vec<Vote>,
        txn_id: TransactionDigest,
        quorum_key: PublicKeyShareVec,
        farmer_id: NodeId,
        txn: TransactionKind,
        quorum_threshold: FarmerQuorumThreshold,
    ) -> Result<()> {
        todo!()
        // let mut vote_shares: HashMap<bool, BTreeMap<NodeIdx, Vec<u8>>> =
        //     HashMap::new();
        // for v in votes.iter() {
        //     if let Some(votes) = vote_shares.get_mut(&v.is_txn_valid) {
        //         votes.insert(v.farmer_node_id, v.signature.clone());
        //     } else {
        //         let sig_shares_map: BTreeMap<NodeIdx, Vec<u8>> =
        //             vec![(v.farmer_node_id, v.signature.clone())]
        //                 .into_iter()
        //                 .collect();
        //         vote_shares.insert(v.is_txn_valid, sig_shares_map);
        //     }
        // }
        //
        // let validated_txns: Vec<_> = self
        //     .validator_core_manager
        //     .validate(
        //         &self.vrrbdb_read_handle.state_store_values(),
        //         vec![txn.clone()],
        //     )
        //     .into_iter()
        //     .collect();
        // let validated = validated_txns.par_iter().any(|x| x.0.id() == txn.id());
        // let most_votes_share = vote_shares
        //     .iter()
        //     .max_by_key(|(_, votes_map)| votes_map.len())
        //     .map(|(key, votes_map)| (*key, votes_map.clone()));
        //
        // if validated {
        //     if let Some((is_txn_valid, votes_map)) = most_votes_share {
        //         let result = sig_provider.generate_quorum_signature(
        //             farmer_quorum_threshold as u16,
        //             votes_map.clone(),
        //         );
        //         if let Ok(threshold_signature) = result {
        //             self.events_tx
        //                 .send(
        //                     Event::CertifiedTxn(JobResult::CertifiedTxn(
        //                         votes.clone(),
        //                         threshold_signature,
        //                         txn_id.clone(),
        //                         farmer_quorum_key.clone(),
        //                         farmer_id.clone(),
        //                         Box::new(txn.clone()),
        //                         is_txn_valid,
        //                     ))
        //                     .into(),
        //                 )
        //                 .await
        //                 .map_err(|err| {
        //                     NodeError::Other(format!(
        //                         "failed to send certified txn: {err}"
        //                     ))
        //                 })?
        //         } else {
        //             error!("Quorum signature generation failed");
        //         }
        //     }
        // } else {
        //     error!("Penalize Farmer for wrong votes by sending Wrong Vote event to CR Quorum");
        // }
    }

    pub fn handle_transaction_certificate_created(
        &mut self,
        votes: Vec<Vote>,
        signature: RawSignature,
        digest: TransactionDigest,
        execution_result: ProgramExecutionOutput,
        farmer_id: NodeId,
        txn: Box<TransactionKind>,
        is_valid: TxnValidationStatus,
    ) {
        //
        // if let JobResult::CertifiedTxn(
        //     votes,
        //     certificate,
        //     txn_id,
        //     farmer_quorum_key,
        //     farmer_id,
        //     txn,
        //     is_txn_valid,
        // ) = job_result
        // {
        //     let vote_receipts = votes
        //         .iter()
        //         .map(|v| VoteReceipt {
        //             farmer_id: v.farmer_id.clone(),
        //             farmer_node_id: v.farmer_node_id,
        //             signature: v.signature.clone(),
        //         })
        //         .collect::<Vec<VoteReceipt>>();
        //
        //     self.quorum_certified_txns.push(QuorumCertifiedTxn::new(
        //         farmer_id,
        //         vote_receipts,
        //         *txn,
        //         certificate,
        //         is_txn_valid,
        //     ));
        //
        //     let _ = self
        //         .certified_txns_filter
        //         .push(&(txn_id, farmer_quorum_key));
        // }
    }
}
