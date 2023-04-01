use crate::{block_builder::BlockBuilder, Miner, conflict_resolver::Resolver};
use block::{ConvergenceBlock, ProposalBlock};


impl BlockBuilder for Miner {
    type BlockType = ConvergenceBlock;
    type RefType = ProposalBlock;

    fn update(&mut self, new_block: &ConvergenceBlock, adjustment: &i128) {
        self.last_block = Some(new_block);
        self.next_epoch_adjustment = adjustment;
    }

    fn build(&self) -> Option<Self::BlockType> {
        let proposals = self.get_references();
        if let Some(proposals) = proposals {

            let resolved = self.resolve(&proposals);
            let txns = self.consolidate_txns(&proposals);
            let claims = self.consolidate_claims(&proposals);
            let ref_hashes = self.get_ref_hashes(&proposals);
            let txn_hash = self.get_txn_hash(&txns);
            let claim_hash = self.get_claim_hash(&claims);
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
            return None
        }
    }


    fn get_references(&self) -> Option<Vec<Self::RefType>> {
        let idx = self.last_block.hash;
        if let Ok(bulldag) = self.dag.read() {
            if let Some(vtx) = bulldag.get_vertex(idx) {
                let p_ids = vtx.get_references();
                let mut proposals = Vec::new();
                p_ids.iter().for_each(|idx| {
                    if let Some(vtx) = bulldag.get_vertex(idx) {
                        match vtx.get_data() {
                            Block::Proposal { ref block } => {
                                proposals.push(vtx);
                            },
                            _ => {}
                        }
                }});
                return proposals
            }
            return None
        }
    }
}

impl Resolver for Miner {
    type ProposalInner = ProposalBlock;
    type Identified = HashMap<TransactionDigest, Conflict>;
    type SourceInner = ConvergenceBlock;
    type BallotInfo = (Claim, RefHash);
    
    /// Identifies conflicts between blocks eligible for inclusion in the 
    /// current round ConvergenceBlock.
    /// It accomplishes this by iterating through all the blocks and 
    /// adding a Conflict struct to a HashMap. The conflict struct 
    /// contains a HashSet with every node that proposed a txn with 
    /// a given transaction digest. It then filters the HashMap to 
    /// only keep Conflicts with more than 1 proposer.
    fn identify<I: IntoIterator<Item=Self::ProposalInner>>(
        &self, proposals: &I
    ) -> &Self::Identified {
        let mut conflicts: ConflictList = HashMap::new();
        proposals.iter().for_each(|block| {
            let mut txn_iter = block.txns.iter();
            let mut proposer = HashSet::new();

            proposer.insert((block.from.clone(), block.hash.clone()));

            while let Some((id, _)) = txn_iter.next() {
                let conflict = Conflict {
                    txn_id: id.to_string(),
                    proposers: proposer.clone(),
                    winner: None,
                };

                conflicts
                    .entry(id.to_string())
                    .and_modify(|e| {
                        e.proposers.insert((block.from.clone(), block.hash.clone()));
                    })
                    .or_insert(conflict);
            }
        });

        conflicts.retain(|_, conflict| conflict.proposers.len() > 1);
        &conflicts
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
    fn resolve<I>(&self, proposals: &I) -> &I 
    where 
        I: IntoIterator<Item=Self::ProposalInner>
    {
        let (curr, prev) = self.split_proposals_by_round(proposals);
        let prev_resolved = self.resolve_earlier(&prev);
        let mut conflicts = self.identify(&curr);

        let proposers = self.get_proposers(&curr); 

        // Construct a BTreeMap of all election results
        let mut election_results = self.get_election_results(proposers); 
        let mut curr_resolved = curr.clone();
        curr_resolved.extend(prev_resolved);

        // Iterate, mutably through all the conflicts identified
        self.append_winner(&mut conflicts, &mut election_results);
        self.resolve_current(&mut curr_resolved, &conflicts);
        &curr_resolved.clone()
    }

    /// Resolves Conflicts between a block that is eligible in this current 
    /// round, i.e. is not already appended to the DAG, but was proposed earlier 
    /// i.e. references a ConvergenceBlock that is not equal to miner.last_block,
    /// and blocks in previous rounds.
    fn resolve_earlier<I: IntoIterator<Item=Self::ProposalInner>>(
        &self, 
        proposals: &I
    ) -> &I {

        let prev_blocks: Vec<ConvergenceBlock> = {
            let nested: Vec<Vec<ConvergenceBlock>> = proposals
                .iter()
                .map(|prop_block| self.get_sources(prop_block))
                .collect();

            nested.into_iter().flatten().collect()
        };

        let mut proposals = proposals.clone();

        // Flatten consolidated transactions from all previous blocks
        let removals: LinkedHashSet<&TxnId> = {
            // Get nested sets of all previous blocks
            let sets: Vec<LinkedHashSet<&TxnId>> = prev_blocks
                .iter()
                .map(|block| {
                    let block_set: Vec<&LinkedHashSet<TxnId>> = {
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

        proposals.retain(|block| block.round != round);

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
    fn get_sources<I>(&self, proposal: &Self::ProposalInner) -> &I 
    where 
        I: IntoIterator<Item = Self::SourceInner>
    {
        // TODO: Handle the case where the reference block is the genesis block
        let source = proposal.ref_block.clone();
        if let Ok(bulldag) = self.dag.read() {

            let source_vtx: Option<&Vertex<Block, String>> = bulldag.get_vertex(source);

            // Get every block between current proposal and proposals source;
            // if the source exists
            let source_refs: Vec<String> = match source_vtx {
                Some(vtx) => bulldag.trace(&vtx, Direction::Reference),
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
                    .map(|idx| chain.get_vertex(idx.to_string())).collect()
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
                    match vtx.get_data() {
                        Block::Convergence { block } => stack.push(block),
                        _ => {},
                    }
                }
            });

            return &stack;
        }

        return &vec![]
    }

    /// Splits proposal blocks into two different proposal blocks
    /// proposal blocks which has a source convergence block that is 
    /// equal to miner.last_block, and proposal blocks with earlier 
    /// round source convergence blocks. 
    fn split_proposals_by_round<I>(&self, proposals: &I) -> (I, I) 
    where 
        I: IntoIterator<Item = Self::ProposalInner>
    {
        let (mut curr, mut prev) = (Vec::new(), Vec::new());
        for block in proposals {
            if block.is_current_round(round) {
                curr.push(block.clone());
            } else {
                prev.push(block.clone());
            }
        }
        (curr.clone(), prev.clone())
    } 

    fn get_proposers<I>(&self, proposals: &I) -> Vec<Self::BallotInfo> 
    where 
        I: IntoIterator<Item = Self::ProposalInner>
    {
        proposals.iter()
            .map(|block| (block.from.clone(), block.hash.clone()))
            .collect()
    }

    fn append_winner(
        &self, 
        conflicts: &mut Self::Identified, 
        election_results: &mut BTreeMap<U256, Self::BallotInfo>
    ) {
        conflicts.iter_mut().for_each(|(_, conflict)| {
            election_results.retain(|(election_results, (claim, ref_hash))| {
                conflict
                    .proposers
                    .contains(&(claim.clone(), ref_hash.clone()))
            });

            // select the first pointer sum and extract the proposal block
            // hash from the pointer sum
            let winner = {

                let mut first: Option<U256, Self::BallotInfo> = election_results.pop_first();
                while let None = first {
                    first = local_pointers.pop_first();
                }

                first
            };
            // save it as the conflict winner
            if let Some((res, (claim, ref_hash))) = winner {
                conflict.winner = Some(ref_hash);
            }
        });
    }

    fn resolve_current<I>(&self, current: &mut I, conflicts: Self::Identified) 
    where
        I: IntoIterator<Item = Self::ProposalInner>
    {
        current.iter_mut().for_each(|block| {
            // Clone conflicts into a mutable variable
            let mut local_conflicts = conflicts.clone();

            // retain only the conflicts that relate to current proposal block
            local_conflicts.retain(|id, _| block.txns.contains_key(id));

            // convert filtered conflicts into an iterator
            let mut conflict_iter = local_conflicts.iter();

            // initialize a hashset to save transactions that current block
            // proposer lost conflict resolution.
            let mut removals = HashSet::new();

            // loop through all the conflicts related to current block
            // and check if the winner is the current block hash
            while let Some((id, conflict)) = conflict_iter.next() {
                if Some(block.hash.clone()) != conflict.winner {
                    // if it does insert into removals, otherwise ignore
                    removals.insert(id.to_string());
                }
            }

            // remove transactions for which current block lost conflict
            // resolution from the current block
            block.txns.retain(|id, _| !removals.contains(id));
        });
    }
}
