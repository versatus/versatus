// This file contains code for creating blocks to be proposed, including the
// genesis block and blocks being mined.

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
    types::SecretKey as SecretKeyBytes,
    Epoch,
    RawSignature,
    GENESIS_EPOCH,
    SECOND,
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
    accountable::Accountable,
    claim::Claim,
    keypair::KeyPair,
    txn::Txn,
    verifiable::Verifiable,
};

#[cfg(mainnet)]
use crate::genesis;
use crate::{
    genesis,
    header::BlockHeader,
    invalid::{BlockError, InvalidBlockErrorReason},
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
    pub inauguration: Option<QuorumPubkeys>,
    pub root_hash: String,
    pub next_root_hash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(C)]
pub struct Conflict {
    pub txn_id: TxnId,
    pub proposers: HashSet<(Claim, RefHash)>,
    pub winner: Option<RefHash>,
}
