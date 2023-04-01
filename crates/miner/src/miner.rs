/// This module is for the creation and operation of a mining unit within a node
/// in the network The miner is the primary way that data replication across all
/// nodes occur The mining of blocks can be thought of as incremental
/// checkpoints in the state.
//FEATURE TAG(S): Block Structure, VRF for Next Block Seed, Rewards
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet, BTreeMap},
    mem, sync::{Arc, RwLock},
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

use crate::{result::MinerError, block_builder::BlockBuilder, conflict_resolver::Resolver};

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
    pub dag: Arc<RwLock<Bulldag<Block, String>>>
}

#[derive(Debug, Clone)]
pub struct Miner {
    secret_key: MinerSk,
    public_key: MinerPk,
    address: Address,
    claim: Claim,
    dag: Arc<RwLock<BullDag<Block, String>>>,
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
            dag: config.dag,
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

    /// Gets a local current timestamp
    pub fn get_timestamp(&self) -> u128 {
        chrono::Utc::now().timestamp() as u128
    }

    pub fn try_mine(
        &mut self
    ) -> Result<Block, MinerError> {
        self.set_status(MinerStatus::Mining);
        if let Some(convergence_block) = self.mine_convergence_block() {
            Ok(Block::Convergence { block: convergence_block })
        } else {
            Err(MinerError::Other("Convergence Block Mining Failed".to_string()))
        }
    }

    pub fn check_claim(&self, winner: U256) -> bool {
        winner == self.claim.hash
    }

    fn set_status(&mut self, status: MinerStatus) {
        self.miner_status = status;
    }

    pub fn mine_convergence_block(
        &self
    ) -> Option<ConvergenceBlock> {
        self.build()
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

    
    #[deprecated(note = "This needs to be moved into a GenesisMiner crate")]
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

    fn consolidate_txns(
        &self, 
        proposals: &Vec<ProposalBlock>
    ) -> ConsolidatedTxns {

        proposals.iter()
            .map(|block| {
                let txn_list = block.txns.iter()
                    .map(|(id, _)| { 
                        id.clone()
                    }).collect();

            (block.hash.clone(), txn_list)
        }).collect()
    }

    fn consolidate_claims(
        &self,
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

    fn get_ref_hashes(&self, proposals: &Vec<ProposalBlock>) -> &Vec<RefHash> {
        proposals.iter().map(|b| {
            b.hash.clone()
        }).collect()
    }

    fn get_txn_hash(&self, txns: &ConsolidatedTxns) -> String {
        let mut txn_hasher = Sha256::new();

        let txns_hash = {
            if let Ok(serialized_txns) = serde_json::to_string(txns) {
                txn_hasher.update(serialized_txns.as_bytes());
            } 
            txn_hasher.finalize()
        };

        format!("{:x}", txn_hash)
    }

    fn get_claim_hash(&self, claims: &ConsolidatedClaims) -> String {

        let mut claim_hasher = Sha256::new();

        let claims_hash = {
            if let Ok(serialized_claims) = serde_json::to_string(claims) {
                claim_hasher.update(serialized_claims.as_bytes());
            }
            claim_hasher.finalize(); 
        };

        format!("{:x}", claims_hash)
    }

    fn build_header(&self, ref_hashes: Vec<RefHash>, txns_hash: String, claims_hash: String) -> Option<BlockHeader> {

        BlockHeader::new(
            self.last_block.clone(),
            ref_hashes.to_owned(),
            self.claim.clone(),
            self.secret_key.clone(),
            txns_hash,
            claims_hash,
            self.next_epoch_adjustment,
        )
    }

    fn hash_block(&self, header: &BlockHeader) -> String {
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

        format!("{:x}", block_hash)
    }
}

impl BlockBuilder for Miner {
    type BlockType = ConvergenceBlock;
    type RefType = ProposalBlock;

    fn update(&mut self, new_block: &ConvergenceBlock, adjustment: &i28) {
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
