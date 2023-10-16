use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};

use block::{
    header::BlockHeader, Block, Conflict, ConflictList, ConvergenceBlock, InnerBlock,
    ProposalBlock, RefHash,
};
use bulldag::vertex::{Direction, Vertex};
use ethereum_types::U256;
use reward::reward::Reward;
use ritelinked::LinkedHashSet;
use vrrb_core::claim::Claim;
use vrrb_core::transactions::TransactionDigest;

use crate::{block_builder::BlockBuilder, conflict_resolver::Resolver, Miner};

impl BlockBuilder for Miner {
    type BlockType = ConvergenceBlock;
    type RefType = ProposalBlock;

    /// Updates the `Miner` instance that it is called on when a new
    /// `ConvergenceBlock` is certified and appended to the `Dag`
    /// We should make sure that the new `ConvergenceBlock` is actually
    /// pulled from the `miner.dag` instance instead of just passing it
    // into this method.
    fn update(
        &mut self,
        last_block: Option<Arc<dyn InnerBlock<Header = BlockHeader, RewardType = Reward>>>,
        adjustment: &i128,
    ) {
        self.last_block = last_block;
        self.next_epoch_adjustment = *adjustment;
    }

    /// Builds and returns a `ConvergenceBlock`
    fn build(&self) -> Option<Self::BlockType> {
        let proposals = self.get_references();
        if let Some(proposals) = proposals {
            let resolved = self.resolve(&proposals, self.get_round(), self.get_seed());
            let txns = self.consolidate_txns(&resolved);
            let claims = self.consolidate_claims(&resolved);
            let ref_hashes = self.get_ref_hashes(&resolved);
            let txns_hash = self.get_txn_hash(&txns);
            let claims_hash = self.get_claim_hash(&claims);
            let header = self.build_header(ref_hashes, txns_hash, claims_hash)?;
            let hash = self.hash_block(&header);

            Some(ConvergenceBlock {
                header,
                txns,
                claims,
                hash,
                certificate: None,
            })
        } else {
            None
        }
    }

    /// Gets all the references currently pointing to the
    /// `miner.last_block` in the DAG, this will return the
    /// `ProposalBlock`s that are pending reference.
    /// Currently this method does not `get` `ProposalBlock`s that
    /// reference earlier `ConvergenceBlock`s but have not yet themselves
    /// been referenced. We need to add this functionality so that
    /// blocks don't get "orphaned"
    fn get_references(&self) -> Option<Vec<Self::RefType>> {
        if let Ok(bulldag) = self.dag.read() {
            let leaf_ids = bulldag.get_leaves();
            let mut proposals = Vec::new();

            leaf_ids.iter().for_each(|leaf| {
                if let Some(vtx) = bulldag.get_vertex(leaf.clone()) {
                    if let Block::Proposal { block } = vtx.get_data() {
                        proposals.push(block);
                    }
                }
            });

            return Some(proposals);
        }

        None
    }

    /// Gets the vertex from the last Convergence (or Genesis) block.
    fn get_last_block_vertex(&self, idx: Option<RefHash>) -> Option<Vertex<Block, String>> {
        if let Some(idx) = idx {
            if let Ok(bulldag) = self.dag.read() {
                if let Some(vtx) = bulldag.get_vertex(idx) {
                    return Some(vtx.clone());
                }
            }
        } else {
            let last_block = self.last_block.clone();
            if let Some(last_block) = last_block {
                let idx = last_block.get_hash();
                if let Ok(bulldag) = self.dag.read() {
                    if let Some(vtx) = bulldag.get_vertex(idx) {
                        return Some(vtx.clone());
                    }
                }
            }
        }
        None
    }
}

impl Resolver for Miner {
    type BallotInfo = (Claim, RefHash);
    type Identified = HashMap<TransactionDigest, Conflict>;
    type Proposal = ProposalBlock;
    type Source = ConvergenceBlock;

    /// Identifies conflicts between blocks eligible for inclusion in the
    /// current round ConvergenceBlock.
    /// It accomplishes this by iterating through all the blocks and
    /// adding a Conflict struct to a HashMap. The conflict struct
    /// contains a HashSet with every node that proposed a txn with
    /// a given transaction digest. It then filters the HashMap to
    /// only keep Conflicts with more than 1 proposer.
    fn identify(&self, proposals: &[Self::Proposal]) -> Self::Identified {
        let mut conflicts: ConflictList = HashMap::new();
        proposals.iter().for_each(|block| {
            let mut proposer = HashSet::new();

            proposer.insert((block.from.clone(), block.hash.clone()));

            for (id, _) in block.txns.iter() {
                let conflict = Conflict {
                    txn_id: id.clone(),
                    proposers: proposer.clone(),
                    winner: None,
                };

                conflicts
                    .entry(id.clone())
                    .and_modify(|e| {
                        e.proposers.insert((block.from.clone(), block.hash.clone()));
                    })
                    .or_insert(conflict);
            }
        });

        conflicts.retain(|_, conflict| conflict.proposers.len() > 1);
        conflicts
    }

    /// Splits proposal blocks by current round and previous rounds
    /// and then attempts to resolve any conflicts between earlier
    /// round proposal blocks (that were not appended to DAG) and
    /// earlier round (from which it was originally proposed).
    /// This is to handle blocks that don't get discovered in time to be
    /// included in the convergence block from the round which they were
    /// originally proposed in.
    ///
    /// After this, the method identifies conflicts, creates an election
    /// results map (`BTreeMap`), elects and appends winners to the conflict.
    /// It then resolves all conflicts in the current round blocks, by removing
    /// the txns associated with the block proposed by the losing party in the
    /// conflict resolution protocol.
    fn resolve(&self, proposals: &[Self::Proposal], round: u128, seed: u64) -> Vec<Self::Proposal> {
        let (mut curr, prev) = self.split_proposals_by_round(proposals);
        let prev_resolved = self.resolve_earlier(&prev, round);
        curr.extend(prev_resolved);
        let mut conflicts = self.identify(&curr);
        let proposers = self.get_proposers(&curr);
        // Construct a BTreeMap of all election results
        let mut election_results = self.get_election_results(&proposers, seed);
        let mut curr_resolved = curr.clone();

        // Iterate, mutably through all the conflicts identified
        self.append_winner(&mut conflicts, &mut election_results);
        self.resolve_current(&mut curr_resolved, &conflicts);
        curr_resolved.clone()
    }

    /// Resolves Conflicts between a block that is eligible in this current
    /// round, i.e. is not already appended to the DAG, but was proposed earlier
    /// i.e. references a ConvergenceBlock that is not equal to
    /// miner.last_block, and blocks in previous rounds.
    fn resolve_earlier(&self, proposals: &[Self::Proposal], round: u128) -> Vec<Self::Proposal> {
        let prev_blocks: Vec<ConvergenceBlock> = {
            let nested: Vec<Vec<ConvergenceBlock>> = proposals
                .iter()
                .map(|prop_block| self.get_sources(prop_block))
                .collect();

            nested.into_iter().flatten().collect()
        };

        let proposals = &mut &(*proposals);

        // Flatten consolidated transactions from all previous blocks
        let removals: LinkedHashSet<&TransactionDigest> = {
            // Get nested sets of all previous blocks
            let sets: Vec<LinkedHashSet<&TransactionDigest>> = prev_blocks
                .iter()
                .map(|block| {
                    let block_set: Vec<&LinkedHashSet<TransactionDigest>> = {
                        block
                            .txns
                            .iter()
                            .map(|(_, txn_id_set)| txn_id_set)
                            .collect()
                    };
                    block_set.into_iter().flatten().collect()
                })
                .collect();

            // Flatten the nested sets
            sets.into_iter().flatten().collect()
        };

        let mut proposals: Vec<&ProposalBlock> = proposals
            .iter()
            .filter(|block| block.round != round)
            .collect();

        let resolved: Vec<ProposalBlock> = proposals
            .iter_mut()
            .map(|block| {
                let mut resolved_block = block.clone();

                resolved_block.txns.retain(|id, _| !&removals.contains(id));

                resolved_block
            })
            .collect();

        resolved
    }

    /// Get every convergence block between the proposal block passed to this
    /// method, and the convergence block that this proposal blocks references,
    /// i.e. this proposal blocks source, and all other blocks in between
    /// before this current block being mined.
    fn get_sources(&self, proposal: &Self::Proposal) -> Vec<Self::Source> {
        // TODO: Handle the case where the reference block is the genesis block
        let source = proposal.ref_block.clone();
        if let Ok(bulldag) = self.dag.read() {
            let source_vtx: Option<&Vertex<Block, String>> = bulldag.get_vertex(source);

            // Get every block between current proposal and proposals source;
            // if the source exists
            let source_refs: Vec<String> = match source_vtx {
                Some(vtx) => bulldag.trace(vtx, Direction::Reference),
                None => {
                    vec![]
                },
            };

            // Get all the vertices corresponding to the references to the
            // proposal blocks source. This will include other proposal blocks
            // between the ProposalBlock's source and the current round.
            // Will need to filter to only retain the convergence blocks
            let ref_vertices: Vec<Option<&Vertex<Block, String>>> = {
                source_refs
                    .iter()
                    .map(|idx| bulldag.get_vertex(idx.to_string()))
                    .collect()
            };

            // Initialize a stack to save ConvergenceBlock vertices to
            // This will where all the ConvergenceBlocks between the
            // Source of ProposalBlock and the current round will be stored
            // and returned to check for conflicts.
            let mut stack = vec![];

            // Iterate through the ref_vertices vector
            // Check whether the ref_vertex is Some or None
            // If it is Some, get the data from the Vertex and
            // match the Block variant
            // If the block variant is a convergence block add it to the stack
            // otherwise ignore it
            ref_vertices.iter().for_each(|opt| {
                if let Some(vtx) = opt {
                    if let Block::Convergence { block } = vtx.get_data() {
                        stack.push(block)
                    }
                }
            });

            return stack;
        }

        vec![]
    }

    /// Takes in a `Vec` of proposer Self::BallotInfo,
    /// which is defined here as `(Claim, RefHash)`, and and gets election
    /// result from it by calling the `claim.get_election_result` method
    /// and passing the current `round` election `seed` into it.
    /// It then builds a `BTreeMap` which is ordered by lowest pointer sums
    /// i.e. the first entry is the winner in the `ConflictResolution`
    /// elections.
    fn get_election_results(
        &self,
        proposers: &[Self::BallotInfo],
        seed: u64,
    ) -> BTreeMap<U256, Self::BallotInfo> {
        proposers
            .iter()
            .map(|(claim, ref_hash)| {
                (
                    claim.get_election_result(seed),
                    (claim.clone(), ref_hash.clone()),
                )
            })
            .collect()
    }

    /// Splits proposal blocks into two different proposal blocks
    /// proposal blocks which has a source convergence block that is
    /// equal to miner.last_block, and proposal blocks with earlier
    /// round source convergence blocks.
    fn split_proposals_by_round(
        &self,
        proposals: &[Self::Proposal],
    ) -> (Vec<Self::Proposal>, Vec<Self::Proposal>) {
        if let Some(last_block) = self.last_block.clone() {
            let (mut curr, mut prev) = (Vec::new(), Vec::new());
            for block in proposals.iter() {
                if block.is_current_round(last_block.get_header().round) {
                    curr.push(block.clone());
                } else {
                    prev.push(block.clone());
                }
            }

            (curr.clone(), prev.clone())
        } else {
            (vec![], vec![])
        }
    }

    /// Takes the `ProposalBlock`s and returns a `Vec` of
    /// `(Claim, RefHash)` i.e. `BallotInfo`
    fn get_proposers(&self, proposals: &[Self::Proposal]) -> Vec<Self::BallotInfo> {
        proposals
            .iter()
            .map(|block| (block.from.clone(), block.hash.clone()))
            .collect()
    }

    /// Adds the winner to the `Conflict` objects in the
    /// `Identified` map.
    fn append_winner(
        &self,
        conflicts: &mut Self::Identified,
        election_results: &mut BTreeMap<U256, Self::BallotInfo>,
    ) {
        conflicts.iter_mut().for_each(|(_, conflict)| {
            election_results.retain(|_, (claim, ref_hash)| {
                conflict
                    .proposers
                    .contains(&(claim.clone(), ref_hash.clone()))
            });

            // select the first pointer sum and extract the proposal block
            // hash from the pointer sum
            let winner = {
                let mut election_iter = election_results.iter();

                let mut first: Option<(&U256, &Self::BallotInfo)> = election_iter.next();
                while first.is_none() {
                    first = election_iter.next();
                }

                first
            }; // <- Remove this extra curly brace

            // save it as the conflict winner
            if let Some((_, (_, ref_hash))) = winner {
                conflict.winner = Some(ref_hash.clone());
            }
        });
    }

    /// Removes conflicting `Txn`s from losing `ProposalBlock`s
    fn resolve_current(&self, current: &mut Vec<Self::Proposal>, conflicts: &Self::Identified) {
        current.iter_mut().for_each(|block| {
            // Clone conflicts into a mutable variable
            let mut local_conflicts = conflicts.clone();

            // retain only the conflicts that relate to current proposal block
            local_conflicts.retain(|id, _| block.txns.contains_key(id));

            // initialize a hashset to save transactions that current block
            // proposer lost conflict resolution.
            let mut removals = HashSet::new();

            // loop through all the conflicts related to current block
            // and check if the winner is the current block hash
            for (id, conflict) in local_conflicts.iter() {
                if Some(block.hash.clone()) != conflict.winner {
                    // if it does insert into removals, otherwise ignore
                    removals.insert(id);
                }
            }

            // remove transactions for which current block lost conflict
            // resolution from the current block
            block.txns.retain(|id, _| !removals.contains(id));
        });
    }
}
