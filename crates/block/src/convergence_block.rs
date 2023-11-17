use primitives::{Epoch, SecretKey as SecretKeyBytes};
use reward::reward::Reward;
#[cfg(mainnet)]
use reward::reward::GENESIS_REWARD;
use ritelinked::{LinkedHashMap, LinkedHashSet};
use serde::{Deserialize, Serialize};
use vrrb_core::claim::Claim;
use vrrb_core::transactions::{TransactionDigest, TransactionKind};

use crate::{
    error::BlockError, header::BlockHeader, Block, BlockHash, Certificate, ConsolidatedClaims,
    ConsolidatedTxns,
};

pub struct MineArgs<'a> {
    pub claim: Claim,
    pub last_block: Block,
    pub txns: LinkedHashMap<String, TransactionKind>,
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

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
#[repr(C)]
pub struct ConvergenceBlock {
    pub header: BlockHeader,
    pub txns: ConsolidatedTxns,
    pub claims: ConsolidatedClaims,
    pub hash: BlockHash,
    pub certificate: Option<Certificate>,
}

impl ConvergenceBlock {
    pub fn append_certificate(&mut self, cert: &Certificate) -> Result<(), BlockError> {
        if self.certificate.is_none() {
            self.certificate = Some(cert.clone());
            return Ok(());
        }

        Err(BlockError::CertificateExists(Block::Convergence {
            block: self.clone(),
        }))
    }

    pub fn txn_id_set(&self) -> LinkedHashSet<&TransactionDigest> {
        self.txns.iter().flat_map(|(_, set)| set).collect()
    }
}
