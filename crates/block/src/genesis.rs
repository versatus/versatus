#[cfg(mainnet)]
use reward::reward::GENESIS_REWARD;
use serde::{Deserialize, Serialize};

#[cfg(mainnet)]
use crate::genesis;
use crate::{header::BlockHeader, BlockHash, Certificate, ClaimList, TxnList};

#[derive(Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
#[repr(C)]
pub struct GenesisBlock<T> {
    pub header: BlockHeader,
    pub txns: TxnList<T>,
    pub claims: ClaimList,
    pub hash: BlockHash,
    pub certificate: Option<Certificate>,
}
