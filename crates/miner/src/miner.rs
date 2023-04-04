/// This module is for the creation and operation of a mining unit within a node
/// in the network The miner is the primary way that data replication across all
/// nodes occur The mining of blocks can be thought of as incremental
/// checkpoints in the state.
//FEATURE TAG(S): Block Structure, VRF for Next Block Seed, Rewards
use std::{
    mem, sync::{Arc, RwLock},
};

use block::{
    block::Block,
    header::BlockHeader,
    invalid::InvalidBlockErrorReason,
    ClaimHash,
    ClaimList,
    ConsolidatedClaims,
    ConsolidatedTxns,
    ConvergenceBlock,
    GenesisBlock,
    ProposalBlock,
    RefHash,
    TxnList, InnerBlock,
};
use bulldag::graph::BullDag;
use primitives::{Address, Epoch, PublicKey, Signature};
use reward::reward::Reward;
use ritelinked::{LinkedHashMap, LinkedHashSet};
use secp256k1::{
    hashes::{sha256 as s256, Hash},
    Message,
};
use serde::{Deserialize, Serialize};
use utils::{create_payload, hash_data};
use vrrb_core::{
    claim::Claim,
    keypair::{MinerPk, MinerSk},
    txn::Txn,
};
use sha2::{Digest, Sha256};
use ethereum_types::U256;

use crate::{result::MinerError, block_builder::BlockBuilder};

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
/// ```
/// use serde::{Deserialize, Serialize};
/// 
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// pub enum MinerStatus {
///     Mining,
///     Waiting,
/// }
///
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MinerStatus {
    Mining,
    Waiting,
}

/// A config struct that is used to consolidate arguments 
/// passed into `Miner::new()` method
///
/// ```
/// use vrrb_core::keypair::{MinerPk, MinerSk};
/// use std::sync::{Arc, RwLock};
/// use bulldag::graph::BullDag;
/// use primitives::Address;
/// use reward::reward::Reward;
/// use block::{Block, header::BlockHeader};
///
/// #[derive(Debug)]
/// pub struct MinerConfig {
///     pub secret_key: MinerSk,
///     pub public_key: MinerPk,
///     pub dag: Arc<RwLock<BullDag<Block, String>>>
/// }
///
#[derive(Debug)]
pub struct MinerConfig {
    pub secret_key: MinerSk,
    pub public_key: MinerPk,
    pub dag: Arc<RwLock<BullDag<Block, String>>>
}


/// Miner struct which exposes methods to mine convergence blocks 
/// via its implementation of the `BlockBuilder` trait, which requires
/// implementation of `Resolver` trait to expose methods to resolve 
/// conflicts between proposal blocks
///
/// ```
/// use vrrb_core::{claim::Claim, keypair::{MinerPk, MinerSk}};
/// use primitives::Address;
/// use miner::{conflict_resolver::Resolver, block_builder::BlockBuilder, miner::MinerStatus};
/// use block::{Block, ConvergenceBlock, header::BlockHeader, InnerBlock};
/// use reward::reward::Reward;
/// use std::sync::{Arc, RwLock};
/// use bulldag::graph::BullDag;
///
/// #[derive(Debug, Clone)]
/// pub struct Miner {
///     secret_key: MinerSk,
///     public_key: MinerPk,
///     address: Address,
///     pub claim: Claim,
///     pub dag: Arc<RwLock<BullDag<Block, String>>>,
///     pub last_block: Option<Arc<dyn InnerBlock<Header = BlockHeader, RewardType = Reward>>>,
///     pub status: MinerStatus,
///     pub next_epoch_adjustment: i128,
/// }
///
#[derive(Debug, Clone)]
pub struct Miner {
    secret_key: MinerSk,
    public_key: MinerPk,
    address: Address,
    pub claim: Claim,
    pub dag: Arc<RwLock<BullDag<Block, String>>>,
    pub last_block: Option<Arc<dyn InnerBlock<Header = BlockHeader, RewardType = Reward>>>,
    pub status: MinerStatus,
    pub next_epoch_adjustment: i128,
}

/// Method Implementations for the Miner Struct
impl Miner {
    /// Creates a new instance of a `Miner`
    ///
    /// # Example
    ///
    /// ```
    /// use vrrb_core::keypair::Keypair;
    /// use primitives::Address;
    /// use miner::miner::{MinerConfig, Miner};
    /// use bulldag::graph::BullDag;
    /// use std::sync::{Arc, RwLock};
    /// 
    /// let keypair = Keypair::random(); 
    /// let (secret_key, public_key) = keypair.miner_kp;
    /// let address = Address::new(public_key.clone());
    /// let dag = Arc::new(RwLock::new(BullDag::new()));
    /// let config = MinerConfig {
    ///     secret_key,
    ///     public_key,
    ///     dag,
    /// };
    ///
    /// let miner = Miner::new(config);
    ///
    /// assert_eq!(miner.address(), address); 
    /// ```
    pub fn new(config: MinerConfig) -> Self {
        let address = Address::new(config.public_key.clone());
        let claim = Claim::new(
            config.public_key.to_string(),
            address.clone().to_string()
        );

        Miner {
            secret_key: config.secret_key,
            public_key: config.public_key,
            address,
            claim,
            dag: config.dag,
            last_block: None,
            status: MinerStatus::Waiting,
            next_epoch_adjustment: 0,
        }
    }

    /// Retrieves the `Address` of the current `Miner` instance
    pub fn address(&self) -> Address {
        self.address.clone()
    }

    /// Retrieves the `PublicKey` of the current `Miner` instance
    pub fn public_key(&self) -> PublicKey {
        self.public_key.clone()
    }

    /// Generates a `Claim` from the `miner.public_key` and `miner.address`
    pub fn generate_claim(&self) -> Claim {
        Claim::new(
            self.public_key().to_string(),
            self.address().to_string(),
        )
    }

    /// Signs a message using the `miner.secret_key`
    pub fn sign_message(&self, msg: Message) -> Signature {
        self.secret_key.sign_ecdsa(msg)
    }

    /// Gets a local current timestamp
    pub fn get_timestamp(&self) -> u128 {
        chrono::Utc::now().timestamp() as u128
    }

    /// Get the next_epoch_adjustment 
    pub fn next_epoch_adjustment(&self) -> i128 {
        self.next_epoch_adjustment
    }

    /// Set the next_epoch_adjustment
    pub fn set_next_epoch_adjustment(&mut self, adjustment: i128) {
        self.next_epoch_adjustment += adjustment;
    }

    /// Attempts to mine a `ConvergenceBlock` using the 
    /// `miner.mine_convergence_block()` method, which in turn uses the 
    /// `<Miner as BlockBuilder>::build()` method
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

    /// Checks if the local `claim.hash` matches the `winner`
    /// This is triggered by the `MiningModule` `Actor` when 
    /// it receives the results from the `ElectionModule<MinerElection, MinerElectionResult>`
    /// `Actor`. If it returns `true` then the local `Miner` calls `try_mine`
    /// which returns a `Block`, and can then subsequently be wrapped in an 
    /// `Event` to be sent to the `BroadcastModule` to send to the proper peer(s)
    /// for certification.
    pub fn check_claim(&self, winner: U256) -> bool {
        winner == self.claim.hash
    }

    /// Sets the current `Miner` instance status to either `MinerStatus::Mining`
    /// or `MinerStatus::Waiting`
    fn set_status(&mut self, status: MinerStatus) {
        self.status = status;
    }

    /// Builds a convergence block using the `<Miner as BlockBuilder>::build()`
    /// method.
    pub fn mine_convergence_block(
        &self
    ) -> Option<ConvergenceBlock> {
        self.build()
    }

    /// This method has been deprecated and will be removed soon
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
            hash: format!("{:x}", hash),
            from,
            signature,
        }
    }
    
    /// This method has been deprecated and will be removed soon
    #[allow(path_statements)]
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
        for (_, _) in txns.iter() {
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
            hash: format!("{:x}", hash),
            from,
            signature,
        })
    }

    
    /// This method has been deprecated and will be removed soon
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
            format!("{:x}", claim_list_hash),
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
            hash: format!("{:x}", block_hash),
            certificate: None,
        };

        Some(genesis)
    }

    /// Consolidates all the `Txn`s in unreferenced `ProposalBlock`s
    /// into a single list of `proposal_block.hash -> txn.id`
    pub(crate) fn consolidate_txns(
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

    /// Consolidates all the `Claims` in the unreferenced `ProposalBlock`s
    /// into a single listt of `proposal_block.hash -> claim.hash`
    pub(crate) fn consolidate_claims(
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

    /// Returns all the unreferenced `ProposalBlock`s hashes in a `Vec`
    pub(crate) fn get_ref_hashes(&self, proposals: &Vec<ProposalBlock>) -> Vec<RefHash> {
        proposals.iter().map(|b| {
            b.hash.clone()
        }).collect()
    }

    /// Hashes and returns a hexadecimal string representation of the hash of 
    /// the consolidated `Txn`s
    pub(crate) fn get_txn_hash(&self, txns: &ConsolidatedTxns) -> String {
        let mut txn_hasher = Sha256::new();

        let txns_hash = {
            if let Ok(serialized_txns) = serde_json::to_string(txns) {
                txn_hasher.update(serialized_txns.as_bytes());
            } 
            txn_hasher.finalize()
        };

        format!("{:x}", txns_hash)
    }

    /// Hashes and returns a hexadecimal string representation of the hash of 
    /// the consolidated `Claim`s
    pub(crate) fn get_claim_hash(&self, claims: &ConsolidatedClaims) -> String {

        let mut claim_hasher = Sha256::new();

        let claims_hash = {
            if let Ok(serialized_claims) = serde_json::to_string(claims) {
                claim_hasher.update(serialized_claims.as_bytes());
            }
            claim_hasher.finalize() 
        };

        format!("{:x}", claims_hash)
    }

    /// Builds a `BlockHeader` for the `ConvergenceBlock` being mined.
    pub(crate) fn build_header(
        &self, 
        ref_hashes: Vec<RefHash>, 
        txns_hash: String, 
        claims_hash: String
    ) -> Option<BlockHeader> {

        if let (Some(block), None) = self.convert_last_block_to_static() {
            return BlockHeader::new(
                block.into(),
                ref_hashes.to_owned(),
                self.claim.clone(),
                self.secret_key.clone(),
                txns_hash,
                claims_hash,
                self.next_epoch_adjustment,
            )
        } 

        if let (None, Some(block)) = self.convert_last_block_to_static() {
            return BlockHeader::new(
                block.into(),
                ref_hashes.to_owned(),
                self.claim.clone(),
                self.secret_key.clone(),
                txns_hash,
                claims_hash,
                self.next_epoch_adjustment
            ) 
        }
        
        return None
    }

    pub(crate) fn convert_last_block_to_static(&self) -> (Option<GenesisBlock>, Option<ConvergenceBlock>) {
        if let Some(block) = self.last_block.clone() {
            if block.is_genesis() {
                return (block.into_static_genesis(), None)
            } else {
                return (None, block.into_static_convergence())
            }
        } else {
            return (None, None)
        }
    }

    /// Hashes the current `ConvergenceBlock` being mined using 
    /// the fields from the `BlockHeader`
    pub(crate) fn hash_block(&self, header: &BlockHeader) -> String {
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

    /// Gets the current election `seed` from the `last_block.header.next_block_seed`
    /// field
    pub(crate) fn get_seed(&self) -> u64 {
        if let Some(last_block) = self.last_block.clone() {
            return last_block.get_header().next_block_seed
        } 

        u32::MAX as u64
    }

    /// Gets the current election `round` from the `last_block.header.round` field
    /// and adds `1` to it.
    pub(crate) fn get_round(&self) -> u128 {

        if let Some(last_block) = self.last_block.clone() {
            return last_block.get_header().round + 1
        }

        0u128
    }

}

