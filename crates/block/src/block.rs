// This file contains code for creating blocks to be proposed, including the
// genesis block and blocks being mined.
#![allow(unused_imports)]
use std::fmt;

use primitives::types::{
    Epoch, RawSignature, SerializedSecretKey as SecretKeyBytes, GENESIS_EPOCH, SECOND,
    VALIDATOR_THRESHOLD,
};

#[cfg(mainnet)]
use reward::reward::GENESIS_REWARD;

use reward::reward::{Reward, NUMBER_OF_BLOCKS_PER_EPOCH};
use ritelinked::{LinkedHashMap, LinkedHashSet};
use serde::{Deserialize, Serialize};
use sha256::digest;
use state::state::NetworkState;
use vrrb_core::{
    accountable::Accountable, claim::Claim, keypair::KeyPair, txn::Txn, verifiable::Verifiable,
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
    pub neighbors: Option<Vec<BlockHeader>>,
    pub height: u128,
    // TODO: replace with Tx Trie Root
    pub txns: LinkedHashMap<String, Txn>,
    // TODO: Replace with Claim Trie Root
    pub claims: LinkedHashMap<String, Claim>,
    pub hash: String,
    pub received_at: Option<u128>,
    pub received_from: Option<String>,
    // TODO: Replace with map of all abandoned claims in the even more than 1 miner is faulty when
    // they are entitled to mine
    pub abandoned_claim: Option<Claim>,

    /// Quorum signature needed for finalizing the block and locking the chain
    pub threshold_signature: Option<RawSignature>,

    /// Epoch for which block was created
    pub epoch: Epoch,

    /// Measurement of utility for the chain
    pub utility: CurrentUtility,

    /// Adjustment For Next Epoch
    pub adjustment_for_next_epoch: Option<NextEpochAdjustment>,
}

impl Block {
    // Returns a result with either a tuple containing the genesis block and the
    // updated account state (if successful) or an error (if unsuccessful)
    pub fn genesis(claim: Claim, secret_key: Vec<u8>, miner: Option<String>) -> Option<Block> {
        // Create the genesis header
        let header = BlockHeader::genesis(0, claim.clone(), secret_key, miner);
        // Create the genesis state hash
        // TODO: Replace with state trie root
        let state_hash = digest(
            format!(
                "{},{}",
                header.last_hash,
                digest("Genesis_State_Hash".as_bytes())
            )
            .as_bytes(),
        );

        // Replace with claim trie
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
            hash: state_hash,
            received_at: None,
            received_from: None,
            abandoned_claim: None,
            threshold_signature: None,
            utility: 0,
            epoch: GENESIS_EPOCH,
            adjustment_for_next_epoch: None,
        };

        // Update the State Trie & Tx Trie with the miner and new block, this will also
        // set the values to the network state. Unwrap the result and assign it
        // to the variable updated_account_state to be returned by this method.

        Some(genesis)
    }

    /// The mine method is used to generate a new block (and an updated account
    /// state with the reward set to the miner wallet's balance), this will
    /// also update the network state with a new confirmed state.
    pub fn mine(args: MineArgs) -> (Option<Block>, NextEpochAdjustment) {
        let claim = args.claim;
        let last_block = args.last_block;
        let txns = args.txns;
        let claims = args.claims;
        let claim_map_hash = args.claim_map_hash;
        let reward = args.reward;
        let network_state = args.network_state;
        let neighbors = args.neighbors;
        let abandoned_claim = args.abandoned_claim;
        let secret_key = args.secret_key;
        let epoch = args.epoch;

        // TODO: Replace with Tx Trie Root
        let txn_hash = {
            let mut txn_vec = vec![];
            txns.iter().for_each(|(_, v)| {
                txn_vec.extend(v.as_bytes());
            });
            digest(&*txn_vec)
        };

        // TODO: Remove there should be no neighbors
        let neighbors_hash = {
            let mut neighbors_vec = vec![];
            if let Some(neighbors) = &neighbors {
                neighbors.iter().for_each(|v| {
                    neighbors_vec.extend(v.as_bytes());
                });
                Some(digest(&*neighbors_vec))
            } else {
                None
            }
        };

        let utility_amount: i128 = txns.iter().map(|x| x.1.get_amount() as i128).sum();
        let mut adjustment_next_epoch = 0;
        let block_utility = if epoch != last_block.epoch {
            adjustment_next_epoch =
                Self::set_next_adjustment_epoch(&last_block, reward, utility_amount);
            utility_amount
        } else {
            utility_amount + last_block.utility
        };

        // TODO: Fix after replacing neighbors and tx hash/claim hash with respective
        // Trie Roots
        let header = BlockHeader::new(
            last_block.clone(),
            reward,
            claim,
            txn_hash,
            claim_map_hash,
            neighbors_hash,
            secret_key,
            epoch == last_block.epoch,
            adjustment_next_epoch,
        );

        // guaranteeing at least 1 second between blocks or whether some other
        // mechanism may serve the purpose better, or whether simply sequencing proposed
        // blocks and allowing validator network to determine how much time
        // between blocks has passed.
        if let Some(time) = header.timestamp.checked_sub(last_block.header.timestamp) {
            if (time / SECOND) < 1 {
                return (None, 0i128);
            }
        } else {
            return (None, 0i128);
        }

        let height = last_block.height + 1;
        let adjustment_next_epoch_opt = if adjustment_next_epoch != 0 {
            Some(adjustment_next_epoch)
        } else {
            None
        };

        let mut block = Block {
            header: header.clone(),
            neighbors,
            height,
            txns,
            claims,
            hash: header.last_hash,
            received_at: None,
            received_from: None,
            abandoned_claim,
            threshold_signature: None,
            utility: block_utility,
            epoch,
            adjustment_for_next_epoch: adjustment_next_epoch_opt,
        };

        // TODO: Replace with state trie
        let mut hashable_state = network_state.clone();

        let hash = hashable_state.hash(&block.txns, block.header.block_reward.clone());
        block.hash = hash;
        (Some(block), adjustment_next_epoch)
    }

    /// If the utility amount is greater than the last block's utility, then the
    /// next adjustment epoch is the utility amount times the gross utility
    /// percentage. Otherwise, the next adjustment epoch is the utility
    /// amount times the negative gross utility percentage
    ///
    /// Arguments:
    ///
    /// * `last_block`: The last block in the chain.
    /// * `reward`: The reward for the current epoch.
    /// * `utility_amount`: The amount of utility that was generated in the last
    ///   epoch.
    ///
    /// Returns:
    ///
    /// The amount of the adjustment for the next epoch.
    fn set_next_adjustment_epoch(
        last_block: &Block,
        reward: &Reward,
        utility_amount: i128,
    ) -> i128 {
        let mut adjustment_next_epoch = if utility_amount > last_block.utility {
            (utility_amount as f64 * GROSS_UTILITY_PERCENTAGE) as i128
        } else {
            (utility_amount as f64 * -GROSS_UTILITY_PERCENTAGE) as i128
        };
        if let Some(adjustment_percentage_previous_epoch) = last_block.adjustment_for_next_epoch {
            if (adjustment_next_epoch / NUMBER_OF_BLOCKS_PER_EPOCH as i128)
                >= adjustment_percentage_previous_epoch * reward.amount as i128
            {
                adjustment_next_epoch = adjustment_percentage_previous_epoch
                    * (reward.amount * NUMBER_OF_BLOCKS_PER_EPOCH) as i128
            };
        };
        adjustment_next_epoch
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_string().as_bytes().to_vec()
    }

    pub fn from_bytes(data: &[u8]) -> Block {
        let mut buffer: Vec<u8> = vec![];

        data.iter().for_each(|x| buffer.push(*x));

        let to_string = String::from_utf8(buffer).unwrap();

        serde_json::from_str::<Block>(&to_string).unwrap()
    }

    // TODO: Consider renaming to `serialize_to_string`
    #[allow(clippy::inherent_to_string_shadow_display)]
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
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

// TODO: Rewrite Verifiable to comport with Masternode Quorum Validation
// Protocol
impl Verifiable for Block {
    type Dependencies = NetworkState;
    type Error = InvalidBlockError;
    type Item = Block;

    fn verifiable(&self) -> bool {
        true
    }

    fn valid(
        &self,
        item: &Self::Item,
        dependencies: &Self::Dependencies,
    ) -> Result<bool, Self::Error> {
        if self.header.block_height > item.header.block_height + 1 {
            return Err(Self::Error::new(
                InvalidBlockErrorReason::BlockOutOfSequence,
            ));
        }

        if self.header.block_height <= item.header.block_height {
            return Err(Self::Error::new(InvalidBlockErrorReason::NotTallestChain));
        }

        if self.header.block_nonce != item.header.next_block_nonce {
            return Err(Self::Error::new(InvalidBlockErrorReason::InvalidBlockNonce));
        }

        if self.header.block_reward.get_amount() != item.header.next_block_reward.get_amount() {
            return Err(Self::Error::new(
                InvalidBlockErrorReason::InvalidBlockReward,
            ));
        }

        if let Some((hash, pointers)) =
            dependencies.get_lowest_pointer(self.header.block_nonce as u128)
        {
            if hash == self.header.claim.hash {
                if let Some(claim_pointer) = self
                    .header
                    .claim
                    .get_pointer(self.header.block_nonce as u128)
                {
                    if pointers != claim_pointer {
                        return Err(Self::Error::new(
                            InvalidBlockErrorReason::InvalidClaimPointers,
                        ));
                    }
                } else {
                    return Err(Self::Error::new(
                        InvalidBlockErrorReason::InvalidClaimPointers,
                    ));
                }
            } else {
                return Err(Self::Error::new(
                    InvalidBlockErrorReason::InvalidClaimPointers,
                ));
            }
        }

        if self.header.last_hash != item.hash {
            return Err(Self::Error::new(InvalidBlockErrorReason::InvalidLastHash));
        }

        if self.header.claim.valid(&None, &(None, None)).is_err() {
            return Err(Self::Error::new(InvalidBlockErrorReason::InvalidClaim));
        }

        Ok(true)
    }

    fn valid_genesis(&self, _dependencies: &Self::Dependencies) -> Result<bool, Self::Error> {
        let genesis_last_hash = digest("Genesis_Last_Hash".as_bytes());
        let genesis_state_hash = digest(
            format!(
                "{},{}",
                genesis_last_hash,
                digest("Genesis_State_Hash".as_bytes())
            )
            .as_bytes(),
        );

        if self.header.block_height != 0 {
            return Err(Self::Error::new(
                InvalidBlockErrorReason::InvalidBlockHeight,
            ));
        }

        if self.header.last_hash != genesis_last_hash {
            return Err(Self::Error::new(InvalidBlockErrorReason::InvalidLastHash));
        }

        if self.hash != genesis_state_hash {
            return Err(Self::Error::new(InvalidBlockErrorReason::InvalidStateHash));
        }

        if self.header.claim.valid(&None, &(None, None)).is_err() {
            return Err(Self::Error::new(InvalidBlockErrorReason::InvalidClaim));
        }

        if KeyPair::verify_ecdsa_sign(
            self.header.signature.clone(),
            self.header.get_payload().as_bytes(),
            self.header.claim.public_key.as_bytes().to_vec(),
        )
        .is_err()
        {
            return Err(Self::Error::new(
                InvalidBlockErrorReason::InvalidBlockSignature,
            ));
        }

        let mut valid_data = true;
        self.txns.iter().for_each(|(_, txn)| {
            let n_valid = txn.validators().iter().filter(|(_, &valid)| valid).count();
            if (n_valid as f64 / txn.validators().len() as f64) < VALIDATOR_THRESHOLD {
                valid_data = false;
            }
        });

        if !valid_data {
            return Err(Self::Error::new(InvalidBlockErrorReason::InvalidTxns));
        }

        Ok(true)
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


