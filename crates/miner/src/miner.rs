/// This module is for the creation and operation of a mining unit within a node
/// in the network The miner is the primary way that data replication across all
/// nodes occur The mining of blocks can be thought of as incremental
/// checkpoints in the state.
//FEATURE TAG(S): Block Structure, VRF for Next Block Seed, Rewards
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet, BTreeMap},
    mem,
};

use block::{
    block::Block,
    header::BlockHeader,
    invalid::InvalidBlockErrorReason,
    ClaimHash,
    ClaimList,
    Conflict,
    ConflictList,
    ConsolidatedClaims,
    ConsolidatedTxns,
    ConvergenceBlock,
    GenesisBlock,
    ProposalBlock,
    RefHash,
    TxnId,
    TxnList,
};
use bulldag::{
    graph::BullDag,
    vertex::{Direction, Vertex},
};
use primitives::{Address, Epoch, PublicKey, SecretKey, Signature};
use reward::reward::Reward;
use ritelinked::{LinkedHashMap, LinkedHashSet};
use secp256k1::{
    hashes::{sha256 as s256, Hash},
    Message,
};
use serde::{Deserialize, Serialize};
use sha256::digest;
use utils::{create_payload, hash_data};
use vrrb_core::{
    claim::Claim,
    keypair::{MinerPk, MinerSk},
    txn::{Txn, TransactionDigest},
};
use sha2::{Digest, Sha256};
use ethereum_types::U256;

use crate::result::MinerError;

// TODO: replace Pool with LeftRightMempool if suitable
//use crate::result::{Result, MinerError};

pub const VALIDATOR_THRESHOLD: f64 = 0.60;
pub const NANO: u128 = 1;
pub const MICRO: u128 = NANO * 1000;
pub const MILLI: u128 = MICRO * 1000;
pub const SECOND: u128 = MILLI * 1000;

// TODO: Consider moving that to genesis_config.yaml
const GENESIS_ALLOWED_MINERS: [&str; 2] = [
    "82104DeE06aa223eC9574a8b2De4fB440630c300",
    "F4ccb23f9A2b10b165965e2a4555EC25615c29BE",
];

/// A basic enum to inform the system whether the current
/// status of the local mining unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MinerStatus {
    Mining,
    Waiting
}

#[derive(Debug)]
pub struct MinerConfig {
    pub secret_key: MinerSk,
    pub public_key: MinerPk,
    pub address: String,
}

#[derive(Debug, Clone)]
pub struct Miner {
    secret_key: MinerSk,
    public_key: MinerPk,
    address: Address,
    claim: Claim,
    last_block: Option<ConvergenceBlock>,
    status: MinerStatus,
}

impl Miner {
    pub fn new(config: MinerConfig) -> Self {
        let public_key = config.public_key.clone();
        let address = Address::new(config.public_key.clone());
        let claim = Claim::new(
            config.public_key,
            address 
        );

        Miner {
            secret_key: config.secret_key,
            public_key: config.public_key,
            address: Address::new(config.public_key.clone()),
            claim,
            last_block: None,
            status: MinerStatus::Waiting,
        }
    }

    pub fn address(&self) -> Address {
        self.address.clone()
    }

    pub fn public_key(&self) -> PublicKey {
        self.public_key.clone()
    }

    pub fn generate_claim(&self) -> Claim {
        Claim::new(
            self.public_key().to_string(),
            self.address().to_string(),
        )
    }

    pub fn sign_message(&self, msg: Message) -> Signature {
        self.secret_key.sign_ecdsa(msg)
    }

    /// Facade method to mine the various available block types
    pub fn mine(&mut self, args: MineArgs) -> Result<Block, MinerError> {
        let now = chrono::Utc::now().timestamp();
        todo!()
    }

    pub fn get_dag(&self) -> Event {
        return Event::GetDag
    }

    pub fn check_claim(&self, winner: U256) -> bool {
        winner == self.claim.hash
    }

    pub fn mine_convergence_block(
        &mut self,
        chain: &BullDag<Block, String>,
        next_epoch_adjustment: i128,
    ) -> Option<ConvergenceBlock> {
        // identify and resolve all the conflicting txns between proposal blocks
        self.miner_status = MinerStatus::Mining;
        
        let proposals = &self.get_proposal_blocks(chain, &self.last_block);
        let resolved_txns = {
            if let Some(last_block) = self.last_block {
                self.get_resolved_txns(
                    proposals, 
                    &last_block, 
                    &last_block.header.round, 
                    chain
                )            
            } else {
                return None 
            }
        };

        let txns: ConsolidatedTxns = self.consolidate_txns(&resolved_txns);
        let claims: ConsolidatedClaims = self.consolidate_claims(&proposals);
        let last_block = self.last_block.clone();
        let claim = self.claim.clone();
        let secret_key = self.secret_key.clone();

        let ref_hashes = proposals.iter().map(|b| {
            b.hash.clone()
        }).collect();

        let mut txn_hasher = Sha256::new();
        let mut claim_hasher = Sha256::new();

        let txns_hash = {
            if let Ok(serialized_txns) = serde_json::to_string(&txn) {
                txn_hasher.update(serialized_txns.as_bytes());
            } 
            txn_hasher.finalize()
        };

        let claims_hash = {
            if let Ok(serialized_claims) = serde_json::to_string(&txn) {
                claim_hasher.update(serialized_claims.as_bytes());
            }
            claim_hasher.finalize(); 
        };

        let txn_hash_string = format!("{:x}", txn_hash);
        let claims_hash_string = format!("{:x}", claims_hash);

        let header = BlockHeader::new(
            last_block.clone(),
            ref_hashes,
            claim,
            secret_key,
            txn_hash,
            claim_list_hash,
            next_epoch_adjustment,
        )?;

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

        self.miner_status = MinerStatus::Waiting;

        Some(ConvergenceBlock {
            header,
            txns,
            claims,
            hash: block_hash,
            certificate: None,
        })
    }

    #[deprecated(note = "Building proposal blocks will be done in Harvester")]
    pub fn mine_proposal_block(
        &self,
        ref_block: RefHash,
        round: u128,
        epoch: Epoch,
        txns: TxnList,
        claims: ClaimList,
        from: Claim,
    ) -> ProposalBlock {

        let payload = create_payload!(round, epoch, txns, claims, from);

        let signature = self.secret_key.sign_ecdsa(payload).to_string();

        let hash = hash_data!(round, epoch, txns, claims, from, signature);

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

    #[deprecated(note = "Building proposal blocks will be done in Harvester")]
    pub fn build_proposal_block(
        &self,
        ref_block: RefHash,
        round: u128,
        epoch: Epoch,
        txns: TxnList,
        claims: ClaimList,
    ) -> Result<ProposalBlock, InvalidBlockErrorReason> {
        let from = self.generate_claim();
        let payload = create_payload!(round, epoch, txns, claims, from);
        let signature = self.secret_key.sign_ecdsa(payload).to_string();
        let hash = hash_data!(round, epoch, txns, claims, from, signature);

        let mut total_txns_size = 0;
        for (_, txn) in txns.iter() {
            total_txns_size += mem::size_of::<Txn>();
            if total_txns_size > 2000 {
                InvalidBlockErrorReason::InvalidBlockSize;
            }
        }

        Ok(ProposalBlock {
            ref_block,
            round,
            epoch,
            txns,
            claims,
            hash,
            from,
            signature,
        })
    }


    pub fn mine_genesis_block(&self, claim_list: ClaimList) -> Option<GenesisBlock> {
        let claim_list_hash = hash_data!(claim_list);
        let seed = 0;
        let round = 0;
        let epoch = 0;

        let claim = self.generate_claim();

        let header = BlockHeader::genesis(
            seed,
            round,
            epoch,
            claim.clone(),
            self.secret_key.clone(),
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
        let header = header;

        let genesis = GenesisBlock {
            header,
            txns,
            claims,
            hash: block_hash,
            certificate: None,
        };

        Some(genesis)
    }

    /// Gets a local current timestamp
    pub fn get_timestamp(&self) -> u128 {
        chrono::Utc::now().timestamp() as u128
    }

    fn get_proposal_blocks(
        &self, 
        bulldag: &BullDag<Block, String>, 
        last_block: ConvergenceBlock
    ) -> Option<Vec<ProposalBlock>> {
        let idx = self.last_block.hash;
        if let Some(vtx) = bulldag.get_vertex(idx) {
            let p_ids = vtx.get_references();
            let mut proposals = Vec::new();
            p_ids.iter().for_each(|idx| {
                if let Some(vtx) = bulldag.get_vertex(idx) {
                    match vtx.get_data() {
                        Block::Proposal { ref block } => {
                            proposals.push(vtx);
                        },
                        _ => { 
                            /*Should throw an error here as 
                             this shouldn't happen, so we 
                             should change return type to 
                             Result<Vec<ProposalBlock>>
                             so that we can propagate 
                             the error.
                            */
                        }
                    }
            }});

            return proposals
        }

        return None
    }

    // Check that conflicts with previous convergence block are removed
    // and there is no winner in current round.
    fn resolve_conflicts(
        &self,
        proposals: &Vec<ProposalBlock>,
        seed: u64,
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
        let prev_resolved = self.resolve_conflicts_prev_rounds(
            round, &prev, chain
        );

        // Identify all conflicts
        let mut conflicts = self.identify_conflicts(&curr);

        let proposers: Vec<(Claim, RefHash)> = curr
            .iter()
            .map(|block| (block.from.clone(), block.hash.clone()))
            .collect();

        // Construct a BTreeMap of all election results
        let mut pointer_sums: BTreeMap<U256, (Claim, RefHash)> = proposers
                .iter()
                .map(|(claim, ref_hash)| {
                    (claim.get_election_result(seed),
                     (claim.clone(), 
                     ref_hash.to_string()))
                })
                .collect();

        // Iterate, mutably through all the conflicts identified
        conflicts.iter_mut().for_each(|(_, conflict)| {
            // clone the pointers sums
            let mut local_pointers = pointer_sums.clone();

            // retain only the pointer sum related to the current conflict
            local_pointers.retain(|(election_results, (claim, ref_hash))| {
                conflict
                    .proposers
                    .contains(&(claim.clone(), ref_hash.clone()))
            });

            // select the first pointer sum and extract the proposal block
            // hash from the pointer sum
            let winner = {

                let mut first: Option<U256,(Claim, RefHash)> = local_pointers.pop_first();

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

    fn resolve_conflicts_prev_rounds(
        &self,
        round: u128,
        proposals: &Vec<ProposalBlock>,
        chain: &BullDag<Block, String>,
    ) -> Vec<ProposalBlock> {
        let prev_blocks: Vec<ConvergenceBlock> = {
            let nested: Vec<Vec<ConvergenceBlock>> = proposals
                .iter()
                .map(|prop_block| self.get_source_blocks(prop_block, chain))
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

    fn identify_conflicts(&self, proposals: &Vec<ProposalBlock>) -> HashMap<TxnId, Conflict> {
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

    fn get_source_blocks(
        &self,
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

    fn get_resolved_txns(
        &self,
        proposals: &Vec<ProposalBlock>,
        last_block: &Block,
        round: &u128,
        chain: &BullDag<Block, String>,
    ) -> Vec<ProposalBlock> {
        match last_block {
            Block::Convergence { ref block } => {
                self.resolve_conflicts(
                    proposals, 
                    &block.header.next_block_seed, 
                    round, 
                    chain
                )
            },
            Block::Genesis { ref block } => {
                self.resolve_conflicts(
                    proposals,
                    &block.head.next_block_seed,
                    round,
                    chain
                )
            },
            _ => return None,
        }
    }

    fn consolidate_txns(
        &self, 
        proposals: &Vec<ProposalBlock>
    ) -> ConsolidatedTxns {

        propsals.iter()
            .map(|block| {
                let txn_list = block.txns.iter()
                    .map(|(id, _)| { 
                        id.clone()
                    }).collect();

            (block.hash.clone(), txn_list)
        }).collect()
    }

    fn consolidate_claims(
        proposals: &Vec<ProposalBlock>
    ) -> ConsolidatedClaims {

        proposals.iter()
            .map(|block| {
                let claim_hashes: LinkedHashSet<ClaimHash> = block
                    .claims
                    .iter()
                    .map(|(claim_hash, _)| claim_hash.clone())
                    .collect();

            (block.hash.clone(), claim_hashes)
        }).collect()
    }
}
