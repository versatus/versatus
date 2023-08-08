use std::collections::HashSet;

use async_trait::async_trait;
use dkg_engine::dkg::DkgGenerator;
use events::{Event, EventMessage, EventPublisher, EventSubscriber, Vote};
use primitives::{NodeId, NodeType};
use telemetry::info;
use theater::{Actor, ActorId, ActorImpl, ActorLabel, ActorState, Handler, TheaterError};
use vrrb_config::{QuorumMember, QuorumMembershipConfig};

use crate::{consensus::ConsensusModule, state_reader::StateReader};

#[async_trait]
impl<S: StateReader + Send + Sync + Clone, K: DkgGenerator + std::fmt::Debug + Send + Sync>
    Handler<EventMessage> for ConsensusModule<S, K>
{
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

                self.dkg_init_dkg_protocol()
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                self.events_tx
                    .send(Event::DkgProtocolInitiated.into())
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },

            // TODO: refactor these event handlers to properly match architecture
            // Event::QuorumElection(header) => {
            //     let claims = self.vrrbdb_read_handle.claim_store_values();
            //
            //     if let Ok(quorum) = self.elect_quorum(claims, header) {
            //         if let Err(err) = self
            //             .events_tx
            //             .send(Event::ElectedQuorum(quorum).into())
            //             .await
            //         {
            //             telemetry::error!("{}", err);
            //         }
            //     }
            // },
            // Event::MinerElection(header) => {
            //     let claims = self.vrrbdb_read_handle.claim_store_values();
            //     let mut election_results: BTreeMap<U256, Claim> =
            //         self.elect_miner(claims, header.block_seed);
            //
            //     let winner = Self::get_winner(&mut election_results);
            //
            //     if let Err(err) = self
            //         .events_tx
            //         .send(Event::ElectedMiner(winner).into())
            //         .await
            //     {
            //         telemetry::error!("{}", err);
            //     }
            // },
            Event::Stop => {
                return Ok(ActorState::Stopped);
            },
            _ => {},
            // // The above code is handling an event of type `Vote` in a Rust
            // // program. It checks the integrity of the vote by
            // // verifying that it comes from the actual voter and prevents
            // // double voting. It then adds the vote to a pool of votes for the
            // // corresponding transaction and farmer quorum key. If
            // // the number of votes in the pool reaches the farmer
            // // quorum threshold, it sends a job to certify the transaction
            // // using the provided signature provider.
            // Event::Vote(vote, farmer_quorum_threshold) => {
            //     //TODO Harvest should check for integrity of the vote by Voter( Does it vote
            //     // truly comes from Voter Prevent Double Voting
            //
            //     if let Some(sig_provider) = self.sig_provider.clone() {
            //         let farmer_quorum_key = hex::encode(vote.quorum_public_key.clone());
            //         if let Some(mut votes) = self
            //             .votes_pool
            //             .get_mut(&(vote.txn.id(), farmer_quorum_key.clone()))
            //         {
            //             let txn_id = vote.txn.id();
            //             if !self
            //                 .certified_txns_filter
            //                 .contains(&(txn_id.clone(), farmer_quorum_key.clone()))
            //             {
            //                 votes.push(vote.clone());
            //                 if votes.len() >= farmer_quorum_threshold {
            //                     let _ = self.sync_jobs_sender.send(Job::CertifyTxn((
            //                         sig_provider,
            //                         votes.clone(),
            //                         txn_id,
            //                         farmer_quorum_key,
            //                         vote.farmer_id.clone(),
            //                         vote.txn,
            //                         farmer_quorum_threshold,
            //                     )));
            //                 }
            //             }
            //         } else {
            //             self.votes_pool
            //                 .insert((vote.txn.id(), farmer_quorum_key), vec![vote]);
            //         }
            //     }
            // },
            // // This certifies txns once vote threshold is reached.
            // Event::CertifiedTxn(job_result) => {
            //     if let JobResult::CertifiedTxn(
            //         votes,
            //         certificate,
            //         txn_id,
            //         farmer_quorum_key,
            //         farmer_id,
            //         txn,
            //         is_txn_valid,
            //     ) = job_result
            //     {
            //         let vote_receipts = votes
            //             .iter()
            //             .map(|v| VoteReceipt {
            //                 farmer_id: v.farmer_id.clone(),
            //                 farmer_node_id: v.farmer_node_id,
            //                 signature: v.signature.clone(),
            //             })
            //             .collect::<Vec<VoteReceipt>>();
            //         self.quorum_certified_txns.push(QuorumCertifiedTxn::new(
            //             farmer_id,
            //             vote_receipts,
            //             *txn,
            //             certificate,
            //             is_txn_valid,
            //         ));
            //         let _ = self
            //             .certified_txns_filter
            //             .push(&(txn_id, farmer_quorum_key));
            //     }
            // },
            //
            // // Mines proposal block after every X seconds.
            // Event::MineProposalBlock(ref_hash, round, epoch, claim) => {
            //     let txns = self.quorum_certified_txns.iter().take(PULL_TXN_BATCH_SIZE);
            //
            //     //Read updated claims
            //     let claim_map = self.vrrbdb_read_handle.claim_store_values();
            //     let claim_list = claim_map
            //         .values()
            //         .map(|claim| (claim.hash, claim.clone()))
            //         .collect();
            //
            //     let txns_list: LinkedHashMap<TransactionDigest, QuorumCertifiedTxn> = txns
            //         .into_iter()
            //         .map(|txn| {
            //             if let Err(err) =
            // self.certified_txns_filter.push(&txn.txn().id.to_string())             {
            //                 telemetry::error!(
            //                     "Error pushing txn to certified txns filter: {}",
            //                     err
            //                 );
            //             }
            //             (txn.txn().id(), txn.clone())
            //         })
            //         .collect();
            //
            //     let proposal_block = ProposalBlock::build(
            //         ref_hash,
            //         round,
            //         epoch,
            //         txns_list,
            //         claim_list,
            //         claim,
            //         self.keypair.get_miner_secret_key(),
            //     );
            //     let _ = self
            //         .broadcast_events_tx
            //         .send(EventMessage::new(
            //             None,
            //             Event::MinedBlock(Block::Proposal {
            //                 block: proposal_block,
            //             }),
            //         ))
            //         .await;
            // },
            // // it sends a job to sign the convergence block using the signature
            // // provider
            // Event::SignConvergenceBlock(block) => {
            //     if let Some(sig_provider) = self.sig_provider.clone() {
            //         let _ = self
            //             .sync_jobs_sender
            //             .send(Job::SignConvergenceBlock(sig_provider, block));
            //     }
            // },
            //
            // // Process the job result of signing convergence block and adds the
            // // partial signature to the cache for certificate generation
            // Event::ConvergenceBlockPartialSign(job_result) => {
            //     if let JobResult::ConvergenceBlockPartialSign(
            //         block_hash,
            //         public_key_share,
            //         partial_signature,
            //     ) = job_result
            //     {
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
            //     }
            // },
            // Event::PeerConvergenceBlockSign(
            //     node_idx,
            //     block_hash,
            //     public_key_share_bytes,
            //     partial_signature,
            // ) => {
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
            // },
            // Event::PrecheckConvergenceBlock(block, last_confirmed_block_header) => {
            //     let claims = block.claims.clone();
            //     let txns = block.txns.clone();
            //     let proposal_block_hashes = block.header.ref_hashes.clone();
            //     let mut pre_check = true;
            //     let mut tmp_proposal_blocks = Vec::new();
            //     if let Ok(dag) = self.dag.read() {
            //         for proposal_block_hash in proposal_block_hashes.iter() {
            //             if let Some(block) = dag.get_vertex(proposal_block_hash.clone()) {
            //                 if let Block::Proposal { block } = block.get_data() {
            //                     tmp_proposal_blocks.push(block.clone());
            //                 }
            //             }
            //         }
            //         for (ref_hash, claim_hashset) in claims.iter() {
            //             match dag.get_vertex(ref_hash.clone()) {
            //                 Some(block) => {
            //                     if let Block::Proposal { block } = block.get_data() {
            //                         for claim_hash in claim_hashset.iter() {
            //                             if !block.claims.contains_key(claim_hash) {
            //                                 pre_check = false;
            //                                 break;
            //                             }
            //                         }
            //                     }
            //                 },
            //                 None => {
            //                     pre_check = false;
            //                     break;
            //                 },
            //             }
            //         }
            //         if pre_check {
            //             for (ref_hash, txn_digest_set) in txns.iter() {
            //                 match dag.get_vertex(ref_hash.clone()) {
            //                     Some(block) => {
            //                         if let Block::Proposal { block } = block.get_data() {
            //                             for txn_digest in txn_digest_set.iter() {
            //                                 if !block.txns.contains_key(txn_digest) {
            //                                     pre_check = false;
            //                                     break;
            //                                 }
            //                             }
            //                         }
            //                     },
            //                     None => {
            //                         pre_check = false;
            //                         break;
            //                     },
            //                 }
            //             }
            //         }
            //     }
            //     if pre_check {
            //         self.broadcast_events_tx
            //             .send(EventMessage::new(
            //                 None,
            //                 Event::CheckConflictResolution((
            //                     tmp_proposal_blocks,
            //                     last_confirmed_block_header.round,
            //                     last_confirmed_block_header.next_block_seed,
            //                     block,
            //                 )),
            //             ))
            //             .await
            //             .map_err(|err| {
            //                 theater::TheaterError::Other(format!(
            //                     "failed to send conflict resolution check: {err}"
            //                 ))
            //             })?
            //     }
            // },
            // Event::NoOp => {},
            // _ => {},
            //
            // Event::AddHarvesterPeer(peer) => {
            //     self.harvester_peers.insert(peer);
            // },
            // Event::RemoveHarvesterPeer(peer) => {
            //     self.harvester_peers.remove(&peer);
            // },
            // /*
            //  *
            //  *
            // Event::SyncNeighbouringFarmerQuorum(peers_details) => {
            //     for (group_public_key, addressess) in peers_details {
            //         self.neighbouring_farmer_quorum_peers
            //             .insert(group_public_key, addressess);
            //     }
            // }
            // *
            // *
            // */
            // // Event  "Farm" fetches a batch of transactions from a transaction mempool and sends
            // // them to scheduler to get it validated and voted
            // Event::Farm => {
            //     let txns = self.tx_mempool.fetch_txns(PULL_TXN_BATCH_SIZE);
            //     let keys: Vec<GroupPublicKey> = self
            //         .neighbouring_farmer_quorum_peers
            //         .keys()
            //         .cloned()
            //         .collect();
            //
            //     let maglev_hash_ring = Maglev::new(keys);
            //
            //     let mut new_txns = vec![];
            //
            //     for txn in txns.into_iter() {
            //         if let Some(group_public_key) = maglev_hash_ring.get(&txn.0.clone()).cloned()
            // {             if group_public_key == self.group_public_key {
            //                 new_txns.push(txn);
            //             } else if let Some(broadcast_addresses) =
            //                 self.neighbouring_farmer_quorum_peers.get(&group_public_key)
            //             {
            //                 let addresses: Vec<SocketAddr> =
            //                     broadcast_addresses.iter().cloned().collect();
            //
            //                 self.broadcast_events_tx
            //                     .send(EventMessage::new(
            //                         None,
            //                         Event::ForwardTxn((txn.1.clone(), addresses.clone())),
            //                     ))
            //                     .await
            //                     .map_err(|err| {
            //                         theater::TheaterError::Other(format!(
            //                             "failed to forward txn {:?} to peers {addresses:?}:
            // {err}",                             txn.1
            //                         ))
            //                     })?
            //             }
            //         } else {
            //             new_txns.push(txn);
            //         }
            //     }
            //
            //     if let Some(sig_provider) = self.sig_provider.clone() {
            //         if let Err(err) = self.sync_jobs_sender.send(Job::Farm((
            //             new_txns,
            //             self.farmer_id.clone(),
            //             self.farmer_node_idx,
            //             self.group_public_key.clone(),
            //             sig_provider,
            //             self.quorum_threshold,
            //         ))) {
            //             telemetry::error!("error sending job to scheduler: {}", err);
            //         }
            //     }
            // },
            // // Receive the Vote from scheduler
            // Event::ProcessedVotes(JobResult::Votes((votes, farmer_quorum_threshold))) => {
            //     for vote in votes.iter().flatten() {
            //         self.broadcast_events_tx
            //             .send(Event::Vote(vote.clone(), farmer_quorum_threshold).into())
            //             .await
            //             .map_err(|err| {
            //                 theater::TheaterError::Other(format!("failed to send vote: {err}"))
            //             })?
            //     }
            // },
            //
            // Event::DkgInitiate => {
            //     let threshold_config = self.dkg_engine.threshold_config.clone();
            //     if self.quorum_type.clone().is_some() {
            //         match self
            //             .dkg_engine
            //             .generate_sync_keygen_instance(threshold_config.threshold as usize)
            //         {
            //             Ok(part_commitment) => {
            //                 if let DkgResult::PartMessageGenerated(node_idx, part) =
            // part_commitment                 {
            //                     if let Ok(part_committment_bytes) = bincode::serialize(&part) {
            //                         let _ = self
            //                             .broadcast_events_tx
            //                             .send(
            //                                 Event::PartMessage(node_idx, part_committment_bytes)
            //                                     .into(),
            //                             )
            //                             .await.map_err(|e| {
            //                                 error!("Error occured while sending part message to
            // broadcast event channel {:?}", e);
            // TheaterError::Other(format!("{e:?}"))                             });
            //                     }
            //                 }
            //             },
            //             Err(_e) => {
            //                 error!("Error occured while generating synchronized keygen instance
            // for node {:?}", self.dkg_engine.node_idx);             },
            //         }
            //     } else {
            //         error!(
            //             "Cannot participate into DKG ,since current node {:?} dint win any Quorum
            // Election",             self.dkg_engine.node_idx
            //         );
            //     }
            //     return Ok(ActorState::Running);
            // },
            // Event::PartMessage(node_idx, part_committment_bytes) => {
            //     let part: bincode::Result<hbbft::sync_key_gen::Part> =
            //         bincode::deserialize(&part_committment_bytes);
            //     if let Ok(part_committment) = part {
            //         self.dkg_engine
            //             .dkg_state
            //             .part_message_store
            //             .entry(node_idx)
            //             .or_insert_with(|| part_committment);
            //     };
            // },
            // Event::AckPartCommitment(sender_id) => {
            //     if self
            //         .dkg_engine
            //         .dkg_state
            //         .part_message_store
            //         .contains_key(&sender_id)
            //     {
            //         let dkg_result = self.dkg_engine.ack_partial_commitment(sender_id);
            //         match dkg_result {
            //             Ok(status) => match status {
            //                 DkgResult::PartMessageAcknowledged => {
            //                     if let Some(ack) = self
            //                         .dkg_engine
            //                         .dkg_state
            //                         .ack_message_store
            //                         .get(&(sender_id, self.dkg_engine.node_idx))
            //                     {
            //                         if let Ok(ack_bytes) = bincode::serialize(&ack) {
            //                             let event = Event::SendAck(
            //                                 self.dkg_engine.node_idx,
            //                                 sender_id,
            //                                 ack_bytes,
            //                             );
            //
            //                             let _ =
            // self.broadcast_events_tx.send(event.into()).await.map_err(|e| {
            //                                 error!("Error occured while sending ack message to
            // broadcast event channel {:?}", e);
            // TheaterError::Other(format!("{e:?}"))                             });
            //                         };
            //                     }
            //                 },
            //                 _ => {
            //                     error!("Error occured while acknowledging partial commitment for
            // node {:?}", sender_id);                 },
            //             },
            //             Err(err) => {
            //                 error!("Error occured while acknowledging partial commitment for node
            // {:?}: Err {:?}", sender_id, err);             },
            //         }
            //     } else {
            //         error!("Part Committment for Node idx {:?} missing", sender_id);
            //     }
            // },
            // Event::HandleAllAcks => {
            //     let result = self.dkg_engine.handle_ack_messages();
            //     match result {
            //         Ok(status) => {
            //             info!("DKG Handle All Acks status {:?}", status);
            //         },
            //         Err(e) => {
            //             error!("Error occured while handling all the acks {:?}", e);
            //         },
            //     }
            // },
            // Event::GenerateKeySet => {
            //     let result = self.dkg_engine.generate_key_sets();
            //     match result {
            //         Ok(status) => {
            //             info!("DKG Completion status {:?}", status);
            //         },
            //         Err(e) => {
            //             error!("Error occured while generating Quorum Public Key {:?}", e);
            //         },
            //     }
            // },
            // Event::HarvesterPublicKey(key_bytes) => {
            //     let result: bincode::Result<PublicKey> = bincode::deserialize(&key_bytes);
            //     if let Ok(harvester_public_key) = result {
            //         self.dkg_engine.harvester_public_key = Some(harvester_public_key);
            //     }
            // },
        }

        Ok(ActorState::Running)
    }
}
