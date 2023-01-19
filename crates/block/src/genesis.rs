use primitives::types::SecretKey as SecretKeyBytes;
#[cfg(mainnet)]
use reward::reward::GENESIS_REWARD;
use ritelinked::LinkedHashMap;
use secp256k1::{hashes::Hash, SecretKey};
use serde::{Deserialize, Serialize};
use sha256::digest;
use utils::{create_payload, hash_data};
use vrrb_core::claim::Claim;

#[cfg(mainnet)]
use crate::genesis;
use crate::{header::BlockHeader, BlockHash, Certificate, ClaimList, TxnList};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(C)]
pub struct GenesisBlock {
    pub header: BlockHeader,
    pub txns: TxnList,
    pub claims: ClaimList,
    pub hash: BlockHash,
    pub certificate: Option<Certificate>,
}
