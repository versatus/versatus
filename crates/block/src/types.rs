// This file contains code for creating blocks to be proposed, including the
// genesis block and blocks being mined.

use std::collections::{HashMap, HashSet};

#[cfg(mainnet)]
use reward::reward::GENESIS_REWARD;
use ritelinked::{LinkedHashMap, LinkedHashSet};
use serde::{Deserialize, Serialize};
use vrrb_core::{
    claim::Claim,
    txn::{Txn, TransactionDigest},
};
use tokio::task::JoinHandle;
use std::error::Error;

#[cfg(mainnet)]
use crate::genesis;

pub const GROSS_UTILITY_PERCENTAGE: f64 = 0.01;
pub const PERCENTAGE_CHANGE_SUPPLY_CAP: f64 = 0.25;
pub const EPOCH_BLOCK: u32 = 30_000_000;

pub type CurrentUtility = i128;
pub type NextEpochAdjustment = i128;
pub type ClaimHash = String;
pub type RefHash = String;
pub type TxnList = LinkedHashMap<TransactionDigest, Txn>;
pub type ClaimList = LinkedHashMap<String, Claim>;
pub type ConsolidatedTxns = LinkedHashMap<RefHash, LinkedHashSet<TransactionDigest>>;
pub type ConsolidatedClaims = LinkedHashMap<RefHash, LinkedHashSet<ClaimHash>>;
pub type BlockHash = String;
pub type QuorumId = String;
pub type QuorumPubkey = String;
pub type QuorumPubkeys = LinkedHashMap<QuorumId, QuorumPubkey>;
pub type ConflictList = HashMap<TransactionDigest, Conflict>;
pub type ResolvedConflicts = Vec<JoinHandle<Result<Conflict, Box<dyn Error>>>>; 

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[repr(C)]
pub struct Certificate {
    pub signature: String,
    pub inauguration: Option<QuorumPubkeys>,
    pub root_hash: String,
    pub next_root_hash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(C)]
pub struct Conflict {
    pub txn_id: TransactionDigest,
    pub proposers: HashSet<(Claim, RefHash)>,
    pub winner: Option<RefHash>,
}
