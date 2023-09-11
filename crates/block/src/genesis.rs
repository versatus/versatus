#[cfg(mainnet)]
use reward::reward::GENESIS_REWARD;
use serde::{Deserialize, Serialize};
use vrrb_core::transactions::TransactionKind;

#[cfg(mainnet)]
use crate::genesis;
use crate::{header::BlockHeader, BlockHash, Certificate, ClaimList, TxnList};

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
#[repr(C)]
pub struct GenesisBlock {
    pub header: BlockHeader,
    pub txns: TxnList,
    pub claims: ClaimList,
    pub hash: BlockHash,
    pub certificate: Option<Certificate>,
}
