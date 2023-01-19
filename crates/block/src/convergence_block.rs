use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    fmt,
};

use bulldag::{
    graph::BullDag,
    index::Index,
    vertex::{Direction, Vertex},
};
use primitives::{
    types::SecretKey as SecretKeyBytes, Epoch, RawSignature, GENESIS_EPOCH, SECOND,
    VALIDATOR_THRESHOLD,
};
#[cfg(mainnet)]
use reward::reward::GENESIS_REWARD;
use reward::reward::{Reward, NUMBER_OF_BLOCKS_PER_EPOCH};
use ritelinked::{LinkedHashMap, LinkedHashSet};
use secp256k1::{
    hashes::{sha256 as s256, Hash},
    Message,
};
use serde::{Deserialize, Serialize};
use sha256::digest;
use utils::{create_payload, hash_data};
use vrrb_core::{
    accountable::Accountable, claim::Claim, keypair::KeyPair, txn::Txn, verifiable::Verifiable,
};

#[cfg(mainnet)]
use crate::genesis;
use crate::{
    genesis,
    header::BlockHeader,
    invalid::{BlockError, InvalidBlockErrorReason},
    Block, BlockHash, Certificate, ClaimHash, Conflict, ConflictList, ConsolidatedClaims,
    ConsolidatedTxns, GenesisBlock, ProposalBlock, RefHash, TxnId,
};

pub struct MineArgs<'a> {
    pub claim: Claim,
    pub last_block: Block,
    pub txns: LinkedHashMap<String, Txn>,
    pub claims: LinkedHashMap<String, Claim>,
    pub claim_list_hash: Option<String>,
    #[deprecated(
        note = "will be removed, unnecessary as last block needed to mine and contains next block reward"
    )]
    pub reward: &'a mut Reward,
    pub abandoned_claim: Option<Claim>,
    pub secret_key: SecretKeyBytes,
    pub epoch: Epoch,
    pub round: u128,
    pub next_epoch_adjustment: i128,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(C)]
pub struct ConvergenceBlock {
    pub header: BlockHeader,
    pub txns: ConsolidatedTxns,
    pub claims: ConsolidatedClaims,
    pub hash: BlockHash,
    pub certificate: Option<Certificate>,
}

impl ConvergenceBlock {
    pub fn mine(
        args: MineArgs,
        proposals: &Vec<ProposalBlock>,
        chain: &BullDag<Block, String>,
    ) -> Option<ConvergenceBlock> {
        // identify and resolve all the conflicting txns between proposal blocks
        let resolved_txns = {
            match args.last_block {
                Block::Convergence { ref block } => ConvergenceBlock::resolve_conflicts(
                    &proposals,
                    block.header.next_block_seed.into(),
                    args.round.clone(),
                    chain,
                ),
                Block::Genesis { ref block } => ConvergenceBlock::resolve_conflicts(
                    &proposals,
                    block.header.next_block_seed.into(),
                    args.round.clone(),
                    chain,
                ),
                _ => return None,
            }
        };

        // Consolidate transactions after resolving conflicts.
        let txns: ConsolidatedTxns = resolved_txns
            .iter()
            .map(|block| {
                let txn_list = block.txns.iter().map(|(id, _)| id.clone()).collect();

                (block.hash.clone(), txn_list)
            })
            .collect();

        //TODO: resolve claim conflicts. This is less important because it
        //cannot lead to double spend
        let claims: ConsolidatedClaims = proposals
            .iter()
            .map(|block| {
                let claim_hashes: LinkedHashSet<ClaimHash> = block
                    .claims
                    .iter()
                    .map(|(claim_hash, _)| claim_hash.clone())
                    .collect();

                (block.hash.clone(), claim_hashes)
            })
            .collect();

        // Get the convergence block from the last round
        let last_block = args.last_block;

        // Get the miner claim
        let claim = args.claim;

        // Get the miner secret key
        let secret_key = args.secret_key;

        // TODO: Calculate the rolling utility and the rolling
        // next epoch adjustment
        let adjustment_next_epoch = args.next_epoch_adjustment;

        // Get all the proposal block hashes
        let ref_hashes = proposals.iter().map(|b| b.hash.clone()).collect();

        // Hash the conflict resolved transactions
        let txn_hash = hash_data!(txns);

        // Hash the claims
        let claim_list_hash = hash_data!(claims);

        // Get the block header for the current block
        let header = BlockHeader::new(
            last_block.clone(),
            ref_hashes,
            claim,
            secret_key,
            txn_hash,
            claim_list_hash,
            adjustment_next_epoch,
        )?;

        // Hash all the header data to get the blockhash
        let block_hash = hash_data!(
            header.ref_hashes,
            header.round,
            header.block_seed,
            header.next_block_seed,
            header.block_height,
            header.timestamp,
            header.txn_hash,
            header.miner_claim,
            header.claim_list_hash,
            header.block_reward,
            header.next_block_reward,
            header.miner_signature
        );

        // Return the ConvergenceBlock
        Some(ConvergenceBlock {
            header,
            txns,
            claims,
            hash: block_hash,
            certificate: None,
        })
    }

    // Check that conflicts with previous convergence block are removed
    // and there is no winner in current round.
    fn resolve_conflicts(
        proposals: &Vec<ProposalBlock>,
        seed: u128,
        round: u128,
        chain: &BullDag<Block, String>,
    ) -> Vec<ProposalBlock> {
        // First, get any/all proposal blocks that are not from current round
        let (curr, prev) = {
            let (mut left, mut right) = (Vec::new(), Vec::new());
            for block in proposals {
                if block.is_current_round(round) {
                    left.push(block.clone());
                } else {
                    right.push(block.clone());
                }
            }

            (left, right)
        };

        // Next get all the prev_round conflicts resolved
        let prev_resolved = ConvergenceBlock::resolve_conflicts_prev_rounds(round, &prev, chain);

        // Identify all conflicts
        let mut conflicts = ConvergenceBlock::identify_conflicts(&curr);

        // create a vector of proposers with the claim and the proposal block
        // hash.
        let proposers: Vec<(Claim, RefHash)> = curr
            .iter()
            .map(|block| (block.from.clone(), block.hash.clone()))
            .collect();

        // calculate the pointer sums for all propsers and save into a vector
        // of thruples with the claim, ref_hash and pointer sum
        let mut pointer_sums: Vec<(Claim, RefHash, Option<u128>)> = {
            proposers
                .iter()
                .map(|(claim, ref_hash)| {
                    (claim.clone(), ref_hash.to_string(), claim.get_pointer(seed))
                })
                .collect()
        };

        // Sort all the pointer sums
        pointer_sums.sort_by(|a, b| match (a.2, b.2) {
            (Some(x), Some(y)) => x.cmp(&y),
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (None, None) => Ordering::Equal,
        });

        // Iterate, mutably through all the conflicts identified
        conflicts.iter_mut().for_each(|(_, conflict)| {
            // clone the pointers sums
            let mut local_pointers = pointer_sums.clone();

            // retain only the pointer sum related to the current conflict
            local_pointers.retain(|(claim, ref_hash, _)| {
                conflict
                    .proposers
                    .contains(&(claim.clone(), ref_hash.clone()))
            });

            // select the first pointer sum and extract the proposal block
            // hash from the pointer sum
            let winner = local_pointers[0].1.clone();

            // save it as the conflict winner
            conflict.winner = Some(winner);
        });

        let mut curr_resolved = curr.clone();
        // Iterate, mutable t hrough the proposal blocks
        curr_resolved.iter_mut().for_each(|block| {
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

        // combine prev_resolved and curr_resolved
        curr_resolved.extend(prev_resolved);

        // return proposal blocks with conflict resolution complete
        curr_resolved.clone()
    }

    fn get_source_blocks(
        block: &ProposalBlock,
        chain: &BullDag<Block, String>,
    ) -> Vec<ConvergenceBlock> {
        // TODO: Handle the case where the reference block is the genesis block
        let source = block.ref_block.clone();
        let source_vtx: Option<&Vertex<Block, String>> = chain.get_vertex(source);

        // Get every block between current proposal and proposals source;
        // if the source exists
        let source_refs: Vec<String> = match source_vtx {
            Some(vtx) => chain.trace(&vtx, Direction::Reference),
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
                .map(|idx| chain.get_vertex(idx.to_string()))
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
                match vtx.get_data() {
                    Block::Convergence { block } => stack.push(block),
                    _ => { /*IGNORE*/ },
                }
            }
        });

        return stack;
    }

    fn resolve_conflicts_prev_rounds(
        round: u128,
        proposals: &Vec<ProposalBlock>,
        chain: &BullDag<Block, String>,
    ) -> Vec<ProposalBlock> {
        let prev_blocks: Vec<ConvergenceBlock> = {
            let nested: Vec<Vec<ConvergenceBlock>> = proposals
                .iter()
                .map(|prop_block| ConvergenceBlock::get_source_blocks(prop_block, chain))
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

    fn identify_conflicts(proposals: &Vec<ProposalBlock>) -> HashMap<TxnId, Conflict> {
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
        conflicts
    }

    pub fn append_certificate(&mut self, cert: Certificate) {
        self.certificate = Some(cert);
    }

    pub fn txn_id_set(&self) -> LinkedHashSet<&TxnId> {
        let sets: Vec<&LinkedHashSet<TxnId>> = self.txns.iter().map(|(_, set)| set).collect();

        sets.into_iter().flatten().collect()
    }
}
