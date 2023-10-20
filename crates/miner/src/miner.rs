use std::net::SocketAddr;
/// This module is for the creation and operation of a mining unit within a node
/// in the network The miner is the primary way that data replication across all
/// nodes occur The mining of blocks can be thought of as incremental
/// checkpoints in the state.
//FEATURE TAG(S): Block Structure, VRF for Next Block Seed, Rewards
use std::sync::{Arc, RwLock};

use block::{
    block::Block, header::BlockHeader, ClaimHash, ClaimList, ConsolidatedClaims, ConsolidatedTxns,
    ConvergenceBlock, GenesisBlock, InnerBlock, ProposalBlock, RefHash,
};
use bulldag::graph::BullDag;
use ethereum_types::U256;
use primitives::{Address, NodeId, PublicKey, Signature};
use reward::reward::Reward;
use ritelinked::{LinkedHashMap, LinkedHashSet};
use secp256k1::Message;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use utils::hash_data;
use vrrb_core::claim::{Claim, ClaimError};
use vrrb_core::keypair::{MinerPublicKey, MinerSecretKey};

use crate::{block_builder::BlockBuilder, result::MinerError};

pub const VALIDATOR_THRESHOLD: f64 = 0.60;
pub const NANO: u128 = 1;
pub const MICRO: u128 = NANO * 1000;
pub const MILLI: u128 = MICRO * 1000;
pub const SECOND: u128 = MILLI * 1000;

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
/// use std::net::SocketAddr;
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
///     pub ip_address:SocketAddr,
///     pub dag: Arc<RwLock<BullDag<Block, String>>>
/// }
#[derive(Debug)]
pub struct MinerConfig {
    pub secret_key: MinerSecretKey,
    pub public_key: MinerPublicKey,
    pub ip_address: SocketAddr,
    pub dag: Arc<RwLock<BullDag<Block, String>>>,
    pub claim: Claim,
}

/// Miner struct which exposes methods to mine convergence blocks
/// via its implementation of the `BlockBuilder` trait, which requires
/// implementation of `Resolver` trait to expose methods to resolve
/// conflicts between proposal blocks
///
/// ```
/// use std::net::SocketAddr;
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
///     pub ip_address:SocketAddr,
///     pub claim: Claim,
///     pub dag: Arc<RwLock<BullDag<Block, String>>>,
///     pub last_block: Option<Arc<dyn InnerBlock<Header = BlockHeader, RewardType = Reward>>>,
///     pub status: MinerStatus,
///     pub next_epoch_adjustment: i128,
/// }
#[derive(Debug, Clone)]
pub struct Miner {
    secret_key: MinerSecretKey,
    public_key: MinerPublicKey,
    address: Address,
    pub ip_address: SocketAddr,
    pub claim: Claim,
    pub dag: Arc<RwLock<BullDag<Block, String>>>,
    pub last_block: Option<Arc<dyn InnerBlock<Header = BlockHeader, RewardType = Reward>>>,
    pub status: MinerStatus,
    pub next_epoch_adjustment: i128,
}

pub type Result<T> = std::result::Result<T, MinerError>;
impl From<ClaimError> for MinerError {
    fn from(error: ClaimError) -> Self {
        match error {
            ClaimError::InvalidSignature => Self::InvalidSignature,
            ClaimError::InvalidPublicKey => Self::InvalidPublicKey,
            ClaimError::Other(details) => Self::Other(details),
        }
    }
}

/// Method Implementations for the Miner Struct
impl Miner {
    /// Creates a new instance of a `Miner`
    ///
    /// # Example
    ///
    /// ```
    /// use std::{
    ///     net::SocketAddr,
    ///     sync::{Arc, RwLock},
    /// };
    ///
    /// use bulldag::graph::BullDag;
    /// use miner::miner::{Miner, MinerConfig};
    /// use primitives::{Address, NodeId};
    /// use vrrb_core::{keypair::Keypair, claim::Claim};
    ///
    /// let keypair = Keypair::random();
    /// let (secret_key, public_key) = keypair.miner_kp;
    /// let address = Address::new(public_key.clone());
    /// let dag = Arc::new(RwLock::new(BullDag::new()));
    /// let ip_address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let signature = Claim::signature_for_valid_claim(
    ///     keypair.miner_kp.1.clone(),
    ///     ip_address.clone(),
    ///     keypair.get_miner_secret_key().secret_bytes().to_vec(),
    /// )
    /// .unwrap();
    /// let claim = Claim::new(
    ///     public_key,
    ///     address.clone(),
    ///     ip_address,
    ///     signature,
    ///     "node_id".to_string(),
    /// ).unwrap();
    /// let config = MinerConfig {
    ///     secret_key,
    ///     public_key,
    ///     ip_address,
    ///     dag,
    ///     claim,
    /// };
    ///
    /// let miner = Miner::new(config, NodeId::default());
    ///
    /// assert_eq!(miner.unwrap().address(), address);
    /// ```
    pub fn new(config: MinerConfig, _node_id: NodeId) -> Result<Self> {
        let address = Address::new(config.public_key);

        let claim = config.claim.clone();

        Ok(Miner {
            secret_key: config.secret_key,
            public_key: config.public_key,
            address,
            ip_address: config.ip_address,
            claim,
            dag: config.dag,
            last_block: None,
            status: MinerStatus::Waiting,
            next_epoch_adjustment: 0,
        })
    }

    /// Retrieves the `Address` of the current `Miner` instance
    pub fn address(&self) -> Address {
        self.address.clone()
    }

    /// Retrieves the `ip_address` of the current `Miner` instance
    pub fn ip_address(&self) -> SocketAddr {
        self.ip_address
    }

    /// Retrieves the `PublicKey` of the current `Miner` instance
    pub fn public_key(&self) -> PublicKey {
        self.public_key
    }

    /// Generates a `Claim` from the `miner.public_key` and `miner.address`
    pub fn generate_claim(&self) -> Result<Claim> {
        let signature = Claim::signature_for_valid_claim(
            self.public_key(),
            self.ip_address(),
            self.secret_key.secret_bytes().to_vec(),
        )
        .map_err(MinerError::from)?;
        let claim = Claim::new(
            self.public_key(),
            self.address(),
            self.ip_address(),
            signature,
            self.claim.node_id.clone(),
        )
        .map_err(MinerError::from)?;
        Ok(claim)
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
    pub fn try_mine(&mut self) -> Result<Block> {
        self.set_status(MinerStatus::Mining);
        if let Some(convergence_block) = self.mine_convergence_block() {
            Ok(Block::Convergence {
                block: convergence_block,
            })
        } else {
            Err(MinerError::Other(
                "Convergence Block Mining Failed".to_string(),
            ))
        }
    }

    /// Checks if the local `claim.hash` matches the `winner`
    /// This is triggered by the `MiningModule` `Actor` when
    /// it receives the results from the `ElectionModule<MinerElection,
    /// MinerElectionResult>` `Actor`. If it returns `true` then the local
    /// `Miner` calls `try_mine` which returns a `Block`, and can then
    /// subsequently be wrapped in an `Event` to be sent to the
    /// `BroadcastModule` to send to the proper peer(s) for certification.
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
    pub fn mine_convergence_block(&self) -> Option<ConvergenceBlock> {
        self.build()
    }

    pub fn mine_genesis_block(&self, claim_list: ClaimList) -> Option<GenesisBlock> {
        let claim_list_hash = hash_data!(claim_list);
        let seed = 0;
        let round = 0;
        let epoch = 0;

        let claim = self.generate_claim().unwrap();

        let header = BlockHeader::genesis(
            seed,
            round,
            epoch,
            claim.clone(),
            self.secret_key,
            format!("{claim_list_hash:x}"),
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
        claims.insert(claim.hash, claim);

        #[cfg(mainnet)]
        let txns = genesis::generate_genesis_txns();

        #[cfg(not(mainnet))]
        let txns = LinkedHashMap::new();
        let header = header;

        let genesis = GenesisBlock {
            header,
            txns,
            claims,
            hash: format!("{block_hash:x}"),
            certificate: None,
        };

        Some(genesis)
    }

    /// Consolidates all the `Txn`s in unreferenced `ProposalBlock`s
    /// into a single list of `proposal_block.hash -> txn.id`
    pub(crate) fn consolidate_txns(&self, proposals: &[ProposalBlock]) -> ConsolidatedTxns {
        proposals
            .iter()
            .map(|block| {
                let txn_list = block.txns.iter().map(|(id, _)| id.clone()).collect();

                (block.hash.clone(), txn_list)
            })
            .collect()
    }

    /// Consolidates all the `Claims` in the unreferenced `ProposalBlock`s
    /// into a single listt of `proposal_block.hash -> claim.hash`
    pub(crate) fn consolidate_claims(&self, proposals: &[ProposalBlock]) -> ConsolidatedClaims {
        proposals
            .iter()
            .map(|block| {
                let claim_hashes: LinkedHashSet<ClaimHash> = block
                    .claims
                    .iter()
                    .map(|(claim_hash, _)| *claim_hash)
                    .collect();

                (block.hash.clone(), claim_hashes)
            })
            .collect()
    }

    /// Returns all the unreferenced `ProposalBlock`s hashes in a `Vec`
    pub(crate) fn get_ref_hashes(&self, proposals: &[ProposalBlock]) -> Vec<RefHash> {
        proposals.iter().map(|b| b.hash.clone()).collect()
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

        format!("{txns_hash:x}")
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

        format!("{claims_hash:x}")
    }

    /// Builds a `BlockHeader` for the `ConvergenceBlock` being mined.
    pub(crate) fn build_header(
        &self,
        ref_hashes: Vec<RefHash>,
        txns_hash: String,
        claims_hash: String,
    ) -> Option<BlockHeader> {
        if let (Some(block), None) = self.convert_last_block_to_static() {
            return BlockHeader::new(
                block.into(),
                ref_hashes,
                self.claim.clone(),
                self.secret_key,
                txns_hash,
                claims_hash,
                self.next_epoch_adjustment,
            );
        }

        if let (None, Some(block)) = self.convert_last_block_to_static() {
            return BlockHeader::new(
                block.into(),
                ref_hashes,
                self.claim.clone(),
                self.secret_key,
                txns_hash,
                claims_hash,
                self.next_epoch_adjustment,
            );
        }

        None
    }

    pub(crate) fn convert_last_block_to_static(
        &self,
    ) -> (Option<GenesisBlock>, Option<ConvergenceBlock>) {
        if let Some(block) = self.last_block.clone() {
            if block.is_genesis() {
                (block.as_static_genesis(), None)
            } else {
                (None, block.as_static_convergence())
            }
        } else {
            (None, None)
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

        format!("{block_hash:x}")
    }

    /// Gets the current election `seed` from the
    /// `last_block.header.next_block_seed` field
    pub(crate) fn get_seed(&self) -> u64 {
        if let Some(last_block) = self.last_block.clone() {
            return last_block.get_header().next_block_seed;
        }

        u32::MAX as u64
    }

    /// Gets the current election `round` from the `last_block.header.round`
    /// field and adds `1` to it.
    pub(crate) fn get_round(&self) -> u128 {
        if let Some(last_block) = self.last_block.clone() {
            return last_block.get_header().round + 1;
        }

        0u128
    }
}

// TODO: figure out how to avoid this
unsafe impl Send for Miner {}
