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
    pub claim: Claim,
    pub dag: Arc<RwLock<BullDag<Block, String>>>,
    pub last_block: Option<ConvergenceBlock>,
    pub status: MinerStatus,
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

    pub(crate) fn get_ref_hashes(&self, proposals: &Vec<ProposalBlock>) -> &Vec<RefHash> {
        proposals.iter().map(|b| {
            b.hash.clone()
        }).collect()
    }

    pub(crate) fn get_txn_hash(&self, txns: &ConsolidatedTxns) -> String {
        let mut txn_hasher = Sha256::new();

        let txns_hash = {
            if let Ok(serialized_txns) = serde_json::to_string(txns) {
                txn_hasher.update(serialized_txns.as_bytes());
            } 
            txn_hasher.finalize()
        };

        format!("{:x}", txn_hash)
    }

    pub(crate) fn get_claim_hash(&self, claims: &ConsolidatedClaims) -> String {

        let mut claim_hasher = Sha256::new();

        let claims_hash = {
            if let Ok(serialized_claims) = serde_json::to_string(claims) {
                claim_hasher.update(serialized_claims.as_bytes());
            }
            claim_hasher.finalize(); 
        };

        format!("{:x}", claims_hash)
    }

    pub(crate) fn build_header(&self, ref_hashes: Vec<RefHash>, txns_hash: String, claims_hash: String) -> Option<BlockHeader> {

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
}

