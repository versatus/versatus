// This file contains code for creating blocks to be proposed, including the
// genesis block and blocks being mined.

use std::{
    collections::{HashMap, HashSet},
    error::Error,
    hash::{Hash, Hasher},
};

use primitives::{NodeId, Signature};
#[cfg(mainnet)]
use reward::reward::GENESIS_REWARD;
use ritelinked::{LinkedHashMap, LinkedHashSet};
use serde::{Deserialize, Serialize};
use signer::engine::QuorumMembers;
use tokio::task::JoinHandle;
use vrrb_core::claim::Claim;
use vrrb_core::transactions::{TransactionDigest, TransactionKind};

#[cfg(mainnet)]
use crate::genesis;

pub const GROSS_UTILITY_PERCENTAGE: f64 = 0.01;
pub const PERCENTAGE_CHANGE_SUPPLY_CAP: f64 = 0.25;
pub const EPOCH_BLOCK: u32 = 30_000_000;

pub type CurrentUtility = i128;
pub type NextEpochAdjustment = i128;
pub type ClaimHash = ethereum_types::U256;
pub type RefHash = String;
pub type TxnList = LinkedHashMap<TransactionDigest, TransactionKind>;
pub type QuorumCertifiedTxnList = LinkedHashMap<TransactionDigest, TransactionKind>;
pub type ClaimList = LinkedHashMap<ClaimHash, Claim>;
pub type ConsolidatedTxns = LinkedHashMap<RefHash, LinkedHashSet<TransactionDigest>>;
pub type ConsolidatedClaims = LinkedHashMap<RefHash, LinkedHashSet<ClaimHash>>;
pub type BlockHash = String;
pub type QuorumId = String;
pub type QuorumPubkey = String;
pub type ConflictList = HashMap<TransactionDigest, Conflict>;
pub type ResolvedConflicts = Vec<JoinHandle<Result<Conflict, Box<dyn Error>>>>;

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
#[repr(C)]
pub struct Certificate {
    pub signatures: Vec<(NodeId, Signature)>,
    pub inauguration: Option<QuorumMembers>,
    pub root_hash: String,
    pub block_hash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[repr(C)]
pub struct Conflict {
    pub txn_id: TransactionDigest,
    pub proposers: HashSet<(Claim, RefHash)>,
    pub winner: Option<RefHash>,
}

#[allow(clippy::derived_hash_with_manual_eq)]
impl Hash for Conflict {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.txn_id.hash(state);
        // Here we sort the elements by their derived hash values to ensure consistent
        // hashing
        let mut sorted_proposers: Vec<_> = self.proposers.iter().collect();
        sorted_proposers.sort_unstable_by(|a, b| {
            let mut hasher_a = std::collections::hash_map::DefaultHasher::new();
            let mut hasher_b = std::collections::hash_map::DefaultHasher::new();

            a.0.hash(&mut hasher_a);
            a.1.hash(&mut hasher_a);

            b.0.hash(&mut hasher_b);
            b.1.hash(&mut hasher_b);

            hasher_a.finish().cmp(&hasher_b.finish())
        });

        for proposer in &sorted_proposers {
            proposer.hash(state);
        }

        self.winner.hash(state);
    }
}

impl Certificate {
    //    pub fn decode_signature(&self) -> Result<RawSignature, FromHexError> {
    //        let signature = hex::decode(self.signatures.clone())?;
    //
    //        Ok(signature)
    //    }
}
