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

impl GenesisBlock {
    // pub fn mine(
    //     claim: Claim,
    //     secret_key: SecretKeyBytes,
    //     claim_list: ClaimList,
    // ) -> Option<GenesisBlock> {
    //     Self::mine_genesis(claim, secret_key, claim_list)
    // }

    // pub fn mine_genesis(
    //     claim: Claim,
    //     secret_key: SecretKey,
    //     claim_list: ClaimList,
    // ) -> Option<GenesisBlock> {
    //     let claim_list_hash = hash_data!(claim_list);
    //     let seed = 0;
    //     let round = 0;
    //     let epoch = 0;
    //
    //     let header = BlockHeader::genesis(
    //         seed,
    //         round,
    //         epoch,
    //         claim.clone(),
    //         secret_key,
    //         claim_list_hash,
    //     );
    //
    //     let block_hash = hash_data!(
    //         header.ref_hashes,
    //         header.round,
    //         header.block_seed,
    //         header.next_block_seed,
    //         header.block_height,
    //         header.timestamp,
    //         header.txn_hash,
    //         header.miner_claim,
    //         header.claim_list_hash,
    //         header.block_reward,
    //         header.next_block_reward,
    //         header.miner_signature
    //     );
    //
    //     let mut claims = LinkedHashMap::new();
    //     claims.insert(claim.clone().public_key, claim);
    //
    //     #[cfg(mainnet)]
    //     let txns = genesis::generate_genesis_txns();
    //
    //     // TODO: Genesis block on local/testnet should generate either a
    //     // faucet for tokens, or fill some initial accounts so that testing
    //     // can be executed
    //
    //     #[cfg(not(mainnet))]
    //     let txns = LinkedHashMap::new();
    //     let header = header;
    //
    //     let genesis = GenesisBlock {
    //         header,
    //         txns,
    //         claims,
    //         hash: block_hash,
    //         certificate: None,
    //     };
    //
    //     Some(genesis)
    // }
}
