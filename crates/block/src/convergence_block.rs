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
    Block,
    BlockHash,
    Certificate,
    ClaimHash,
    Conflict,
    ConflictList,
    ConsolidatedClaims,
    ConsolidatedTxns,
    GenesisBlock,
    ProposalBlock,
    RefHash,
    TxnId,
};

pub struct MineArgs<'a> {
    pub claim: Claim,
    pub last_block: Block,
    pub txns: LinkedHashMap<String, Txn>,
    pub claims: LinkedHashMap<String, Claim>,
    pub claim_list_hash: Option<String>,
    #[deprecated(
        note = "will be removed, unnecessary as last block needed to mine and contains next block reward"
    )]
    pub reward: &'a mut Reward,
    pub abandoned_claim: Option<Claim>,
    pub secret_key: SecretKeyBytes,
    pub epoch: Epoch,
    pub round: u128,
    pub next_epoch_adjustment: i128,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(C)]
pub struct ConvergenceBlock {
    pub header: BlockHeader,
    pub txns: ConsolidatedTxns,
    pub claims: ConsolidatedClaims,
    pub hash: BlockHash,
    pub certificate: Option<Certificate>,
}

impl ConvergenceBlock {
    pub fn append_certificate(&mut self, cert: Certificate) {
        self.certificate = Some(cert);
    }

    pub fn txn_id_set(&self) -> LinkedHashSet<&TxnId> {
        self.txns.iter().flat_map(|(_, set)| set).collect()
    }
}
