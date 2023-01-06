// This file contains code for creating blocks to be proposed, including the
// genesis block and blocks being mined.
#![allow(unused_imports)]
use std::fmt;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use primitives::{
    Epoch, 
    RawSignature, 
    SecretKey as SecretKeyBytes, 
    GENESIS_EPOCH, 
    SECOND, 
    VALIDATOR_THRESHOLD,
};

#[cfg(mainnet)]
use reward::reward::GENESIS_REWARD;

use reward::reward::{Reward, NUMBER_OF_BLOCKS_PER_EPOCH};
use ritelinked::{LinkedHashMap, LinkedHashSet};
use serde::{Deserialize, Serialize};
use vrrb_core::{
    accountable::Accountable, 
    claim::Claim, 
    keypair::KeyPair, 
    txn::Txn, 
    verifiable::Verifiable,
};

use secp256k1::{hashes::{sha256 as s256, Hash}, Message};
use sha256::digest;
use utils::{create_payload, hash_data};
use bulldag::{graph::BullDag, index::Index, vertex::{Vertex, Direction}};

#[cfg(mainnet)]
use crate::genesis;
use crate::{
    header::BlockHeader,
    invalid::{InvalidBlockError, InvalidBlockErrorReason}, genesis,
};

pub const GROSS_UTILITY_PERCENTAGE: f64 = 0.01;
pub const PERCENTAGE_CHANGE_SUPPLY_CAP: f64 = 0.25;
pub const EPOCH_BLOCK: u32 = 30_000_000;

pub type CurrentUtility = i128;
pub type NextEpochAdjustment = i128;
pub type TxnId = String;
pub type ClaimHash = String;
pub type RefHash = String;
pub type TxnList = LinkedHashMap<TxnId, Txn>;
pub type ClaimList = LinkedHashMap<ClaimHash, Claim>;
pub type ConsolidatedTxns = LinkedHashMap<RefHash, LinkedHashSet<TxnId>>;
pub type ConsolidatedClaims = LinkedHashMap<RefHash, LinkedHashSet<ClaimHash>>;
pub type BlockHash = String;
pub type QuorumId = String;
pub type QuorumPubkey = String;
pub type QuorumPubkeys = LinkedHashMap<QuorumId, QuorumPubkey>;
pub type ConflictList = HashMap<TxnId, Conflict>;
 


#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(C)]
pub struct Certificate {
    pub signature: String,
    pub inauguartion: Option<QuorumPubkeys>,
    pub root_hash: String,
    pub next_root_hash: String
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(C)]
pub struct GenesisBlock {
    pub header: BlockHeader,
    pub txns: TxnList,
    pub claims: ClaimList,
    pub hash: BlockHash,
    pub certificate: Option<Certificate>
}

impl GenesisBlock {
    pub fn mine_genesis(
        claim: Claim, 
        secret_key: SecretKeyBytes, 
        miner_pubkey: String,
        claim_list: ClaimList,
    ) -> Option<GenesisBlock> {

        let claim_list_hash = hash_data!(claim_list);

        let header = BlockHeader::genesis(
            0, 
            0,
            claim.clone(), 
            secret_key, 
            claim_list_hash,
        );

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

        let mut claims = LinkedHashMap::new();
        claims.insert(claim.clone().public_key, claim);

        #[cfg(mainnet)]
        let txns = genesis::generate_genesis_txns();

        // TODO: Genesis block on local/testnet should generate either a 
        // faucet for tokens, or fill some initial accounts so that testing 
        // can be executed
        
        #[cfg(not(mainnet))]
        let txns = LinkedHashMap::new();

        let genesis = GenesisBlock {
            header,
            txns,
            claims,
            hash: block_hash,
            certificate: None
        };

        Some(genesis)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(C)]
pub struct ConvergenceBlock {
    pub header: BlockHeader,
    pub txns: ConsolidatedTxns,
    pub claims: ConsolidatedClaims,
    pub hash: BlockHash,
    pub certificate: Option<Certificate>
}

impl ConvergenceBlock {

    pub fn mine(
        args: MineArgs,
        proposals: &Vec<ProposalBlock>,
        chain: &BullDag<Block, String>
    ) -> ConvergenceBlock {
        // identify and resolve all the conflicting txns between proposal blocks
        let resolved_txns = ConvergenceBlock::resolve_conflicts(
            &proposals, 
            args.last_block.header.next_block_seed.into(),
            args.round.clone(),
            chain
        );
        
        // Consolidate transactions after resolving conflicts.
        let txns: ConsolidatedTxns = resolved_txns.iter().map(|block| {
            let txn_list = block.txns.iter().map(|(id, _)| {
                id.clone()
            }).collect();

            (block.hash.clone(), txn_list)
        }).collect();

        //TODO: resolve claim conflicts. This is less important because it 
        //cannot lead to double spend
        let claims: ConsolidatedClaims = proposals.iter().map(|block| {
            let claim_hashes: LinkedHashSet<ClaimHash> = block.claims.iter().map(
                |(claim_hash, claim)| {
                    claim_hash.clone()
            }).collect();

            (block.hash.clone(), claim_hashes)
        }).collect();

        // Get the convergence block from the last round
        let last_block = args.last_block;

        // Get the miner claim 
        let claim = args.claim;

        // Get the block reward 
        let mut reward = args.reward;

        // Get the miner secret key
        let secret_key = args.secret_key; 

        // TODO: Calculate the rolling utility and the rolling
        // next epoch adjustment 
        let adjustment_next_epoch = 0;

        // Get the current round 
        let round = args.round;

        // Check whether this block is the epoch changing block 
        let epoch_change = {
            if last_block.header.block_height % EPOCH_BLOCK as u128 == 0 {
                true
            } else {
                false
            }
        };

        // Get all the proposal block hashes
        let ref_hashes = proposals.iter().map(|b| {b.hash.clone()}).collect();

        // Hash the conflict resolved transactions
        let txn_hash = hash_data!(txns);
        
        // Hash the claims
        let claim_list_hash = hash_data!(claims);

        // Get the block header for the current block 
        let header = BlockHeader::new(
            last_block,
            ref_hashes, 
            round,
            &mut reward, 
            claim, 
            secret_key,
            txn_hash,
            claim_list_hash,
            epoch_change,
            adjustment_next_epoch
        );

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
        ConvergenceBlock {
            header,
            txns,
            claims,
            hash: block_hash,
            certificate: None
        }
    }

    // TODO: Check that conflicts with previous convergence block are removed
    // and there is no winner in current round. 
    fn resolve_conflicts(
        proposals: &Vec<ProposalBlock>,
        seed: u128,
        round: u128,
        chain: &BullDag<Block, String>
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
        let prev_resolved = ConvergenceBlock::resolve_conflicts_prev_rounds(
            round, &prev, chain
        );

        // Identify all conflicts
        let mut conflicts = ConvergenceBlock::identify_conflicts(&curr);

        // create a vector of proposers with the claim and the proposal block 
        // hash.
        let proposers: Vec<(Claim, RefHash)> = curr.iter().map(|block| {
            (block.from.clone(), block.hash.clone())
        }).collect();
        
        // calculate the pointer sums for all propsers and save into a vector 
        // of thruples with the claim, ref_hash and pointer sum
        let mut pointer_sums: Vec<(Claim, RefHash, Option<u128>)> = {
            proposers.iter().map(|(claim, ref_hash)| {

                (claim.clone(), 
                 ref_hash.to_string(), 
                 claim.get_pointer(seed)) 

            }).collect()
        };

        // Sort all the pointer sums
        pointer_sums.sort_by(|a, b| {
            match (a.2, b.2) {
                (Some(x), Some(y)) => x.cmp(&y),
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (None, None) => Ordering::Equal,
            }
        });

        // Iterate, mutably through all the conflicts identified 
        conflicts.iter_mut().for_each(|(id, conflict)| {
            // clone the pointers sums
            let mut local_pointers = pointer_sums.clone();

            // retain only the pointer sum related to the current conflict
            local_pointers.retain(|(claim, ref_hash, pointer_sum)| {
                conflict.proposers.contains(&(claim.clone(), ref_hash.clone())) 
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
            local_conflicts.retain(|id, conflict| {
                block.txns.contains_key(id)
            });

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
            block.txns.retain(|id, _| {
                !removals.contains(id)
            });
        });
        
        // combine prev_resolved and curr_resolved
        curr_resolved.extend(prev_resolved);

        // return proposal blocks with conflict resolution complete
        curr_resolved.clone()
    }

    fn get_source_blocks(
        block: &ProposalBlock, 
        chain: &BullDag<Block, String>
    ) -> Vec<ConvergenceBlock> {

        // TODO: Handle the case where the reference block is the genesis block
        let source = block.ref_block.clone();
        let source_vtx: Option<&Vertex<Block, String>> = chain.get_vertex(source);

        // Get every block between current proposal and proposals source;
        // if the source exists
        let source_refs: Vec<String> = match source_vtx {
            Some(vtx) => {
                chain.trace(&vtx, Direction::Reference) 
            },
            None => { vec![] }
        };
        
        // Get all the vertices corresponding to the references to the 
        // proposal blocks source. This will include other proposal blocks 
        // between the ProposalBlock's source and the current round. 
        // Will need to filter to only retain the convergence blocks 
        let mut ref_vertices: Vec<Option<&Vertex<Block, String>>> = {
            source_refs.iter().map(|idx| {
                chain.get_vertex(idx.to_string())
            }).collect()
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
                    _ => {/*IGNORE*/}
                }
            }
        });

        return stack
    }

    fn resolve_conflicts_prev_rounds(
        round: u128,
        proposals: &Vec<ProposalBlock>, 
        chain: &BullDag<Block, String>
    ) -> Vec<ProposalBlock> {
        let mut prev_blocks: Vec<ConvergenceBlock> = {
            let nested: Vec<Vec<ConvergenceBlock>> = proposals.iter().map(
                |prop_block| {
                    ConvergenceBlock::get_source_blocks(prop_block, chain)
                }
            ).collect(); 

            nested.into_iter().flatten().collect()
        };
        
        let mut proposals = proposals.clone();

        // Flatten consolidated transactions from all previous blocks
        let mut removals: LinkedHashSet<&TxnId> = {
            // Get nested sets of all previous blocks
            let sets: Vec<LinkedHashSet<&TxnId>> = prev_blocks.iter().map(
                |block| {
                    let block_set: Vec<&LinkedHashSet<TxnId>> = {
                        block.txns.iter().map(|(_, txn_id_set)| {
                            txn_id_set
                        }
                    ).collect()
                };
                block_set.into_iter().flatten().collect()
            }).collect();

            // Flatten the nested sets
            sets.into_iter().flatten().collect()
        };
        
        proposals.retain(|block| { block.round != round });

        let resolved: Vec<ProposalBlock> = proposals.iter_mut().map(|block| {

            let mut resolved_block = block.clone();

            resolved_block.txns.retain(|id, _| {
                !&removals.contains(id)
            });

            resolved_block
        }).collect();

        resolved
        
    }

    fn identify_conflicts(
        proposals: &Vec<ProposalBlock>
    ) -> HashMap<TxnId, Conflict> {

        let mut conflicts: ConflictList = HashMap::new();

        proposals.iter().for_each(|block| {

            let mut txn_iter = block.txns.iter();
            
            let mut proposer = HashSet::new();
            proposer.insert((block.from.clone(), block.hash.clone()));

            while let Some((id, txn)) = txn_iter.next() {

                let conflict = Conflict {
                    txn_id: id.to_string(),
                    proposers: proposer.clone(),
                    winner: None
                }; 

                conflicts.entry(id.to_string()).and_modify(|e| {
                    e.proposers.insert(
                        (block.from.clone(), block.hash.clone())
                    );
                }).or_insert(
                    conflict
                );
            }
        });

        conflicts.retain(|id, conflict| conflict.proposers.len() > 1);
        conflicts
    }

    pub fn append_certificate(&mut self, cert: Certificate) {
       self.certificate = Some(cert); 
    }

    pub fn txn_id_set(&self) -> LinkedHashSet<&TxnId> {
        let sets: Vec<&LinkedHashSet<TxnId>> = self.txns.iter().map(
            |(ref_hash, set)| {
                set 
            }
        ).collect();

        sets.into_iter().flatten().collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(C)]
pub struct Conflict {
    pub txn_id: TxnId,
    pub proposers: HashSet<(Claim, RefHash)>,
    pub winner: Option<RefHash>
}

pub struct MineArgs<'a> {
    pub claim: Claim,
    pub last_block: ConvergenceBlock,
    pub txns: LinkedHashMap<String, Txn>,
    pub claims: LinkedHashMap<String, Claim>,
    pub claim_map_hash: Option<String>,
    pub reward: &'a mut Reward,
    pub neighbors: Option<Vec<BlockHeader>>,
    pub abandoned_claim: Option<Claim>,
    pub secret_key: SecretKeyBytes,
    pub epoch: Epoch,
    pub round: u128
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(C)]
pub struct ProposalBlock {
    pub ref_block: RefHash,
    pub round: u128,
    pub epoch: Epoch,
    pub txns: TxnList,
    pub claims: ClaimList,
    pub from: Claim,
    pub hash: BlockHash,
    pub signature: String,
}

impl ProposalBlock { 
    pub fn build(
        ref_block: RefHash,
        round: u128, 
        epoch: Epoch, 
        txns: TxnList, 
        claims: ClaimList, 
        from: Claim,
        secret_key: SecretKeyBytes,
    ) -> ProposalBlock {
        let payload = create_payload!(
            round, epoch, txns, claims, from
        );
        
        let signature = secret_key.sign_ecdsa(payload).to_string();
        
        let hash = hash_data!(
            round, epoch, txns, claims, from, signature
        );

        ProposalBlock {
            ref_block,
            round,
            epoch,
            txns,
            claims,
            hash,
            from,
            signature,
        }
    }

    pub fn is_current_round(&self, round: u128) -> bool {
        self.round == round
    }

    pub fn remove_confirmed_txs(&mut self, prev_blocks: Vec<ConvergenceBlock>) {
        let sets: Vec<LinkedHashSet<&TxnId>> = {
            prev_blocks.iter().map(|block| {
                block.txn_id_set()
            }).collect()
        };
        
        let prev_block_set: LinkedHashSet<&TxnId> = {
            sets.into_iter().flatten().collect()
        }; 
       
        let curr_txns = self.txns.clone();

        let curr_set: LinkedHashSet<&TxnId> = {
            curr_txns.iter().map(|(id, _)| {
                id
            }).collect()
        };
        
        let prev_confirmed: LinkedHashSet<TxnId> = {
            let intersection = curr_set.intersection(&prev_block_set);
            intersection.into_iter().map(|id| { id.to_string() }).collect()
        };
        
        self.txns.retain(|id, txn| { prev_confirmed.contains(id) });
    }

    fn txn_id_set(&self) -> LinkedHashSet<TxnId> {
        self.txns.iter().map(|(id, _)| {
            id.clone()
        }).collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(C)]
pub enum Block {
    Convergence {
        block: ConvergenceBlock
    },
    Proposal {
        block: ProposalBlock
    },
    Genesis {
        block: GenesisBlock
    }
}

impl Block {
    pub fn is_convergence(&self) -> bool {
        match self {
            Block::Convergence { .. } => return true,
            _ => return false
        }
    }

    pub fn is_proposal(&self) -> bool {
        match self {
            Block::Proposal { .. } => return true,
            _ => return false
        }
    }

    pub fn is_genesis(&self) -> bool {
        match self {
            Block::Genesis { .. } => return true,
            _ => return false
        }
    }
}

impl fmt::Display for ConvergenceBlock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "ConvergenceBlock(\n \
            header: {:?},\n",
            self.header
        )
    }
}

//TODO: impl fmt::Display for ProposalBlock & GenesisBlock
impl From<ConvergenceBlock> for Block {
    fn from(item: ConvergenceBlock) -> Block {
        return Block::Convergence { block: item }
    }
}

impl From<ProposalBlock> for Block {
    fn from(item: ProposalBlock) -> Block {
        return Block::Proposal { block: item }
    }
}

impl From<GenesisBlock> for Block {
    fn from(item: GenesisBlock) -> Block {
        return Block::Genesis { block: item }
    } 
}


