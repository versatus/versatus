use primitives::{Epoch, SecretKey as SecretKeyBytes};
use ritelinked::LinkedHashSet;
use secp256k1::{
    hashes::{sha256 as s256, Hash},
    Message,
};
use serde::{Deserialize, Serialize};
use sha256::digest;
use utils::{create_payload, hash_data};
use vrrb_core::{claim::Claim, txn::{Txn, TransactionDigest}};

use crate::{BlockHash, ClaimList, ConvergenceBlock, RefHash, TxnList};

/// A Block type that goes between two ConvergenceBlocks in the 
/// VRRB Dag.
///
/// ```
/// use serde::{Serialize, Deserialize};
/// use block::{BlockHash, RefHash, TxnList};
/// use primitives::Epoch;
///
/// #[derive(Clone, Debug, Serialize, Deserialize)]
/// #[repr(C)]
/// pub struct ProposalBlock {
///     pub ref_block: RefHash;
///     pub round: u128,
///     pub epoch: Epoch,
///     pub txns: TxnList,
///     pub claims: ClaimList,
///     pub from: Claim,
///     pub hash: BlockHash,
///     pub signature: String,
/// }
/// ```
///
#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(C)]
pub struct ProposalBlock {
    pub ref_block: RefHash,
    pub round: u128,
    pub epoch: Epoch,
    pub txns: TxnList,
    pub claims: ClaimList,
    pub from: Claim,
    pub hash: BlockHash,
    pub signature: String,
}

impl ProposalBlock {
    pub fn build(
        ref_block: RefHash,
        round: u128,
        epoch: Epoch,
        txns: TxnList,
        claims: ClaimList,
        from: Claim,
        secret_key: SecretKeyBytes,
    ) -> ProposalBlock {
        let payload = create_payload!(round, epoch, txns, claims, from);
        let signature = secret_key.sign_ecdsa(payload).to_string();
        let hashable_txns: Vec<(String, Txn)> = {
            txns.clone().iter().map(|(k, v)| {
                (k.digest_string(), v.clone())
            }).collect()
        };
        let hash = hash_data!(round, epoch, hashable_txns, claims, from, signature);
        let hash_string = format!("{:x}", hash);

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

    pub fn is_current_round(&self, round: u128) -> bool {
        self.round == round
    }

    pub fn remove_confirmed_txs(&mut self, prev_blocks: Vec<ConvergenceBlock>) {
        let sets: Vec<LinkedHashSet<&TransactionDigest>> =
            { prev_blocks.iter().map(|block| block.txn_id_set()).collect() };

        let prev_block_set: LinkedHashSet<&TransactionDigest> = { sets.into_iter().flatten().collect() };

        let curr_txns = self.txns.clone();

        let curr_set: LinkedHashSet<&TransactionDigest> = { curr_txns.iter().map(|(id, _)| id).collect() };

        let prev_confirmed: LinkedHashSet<&TransactionDigest> = {
            let intersection = curr_set.intersection(&prev_block_set);
            intersection.into_iter().map(|id| id.clone()).collect()
        };

        self.txns.retain(|id, _| prev_confirmed.contains(id));
    }

    pub fn txn_id_set(&self) -> LinkedHashSet<TransactionDigest> {
        self.txns.iter().map(|(id, _)| id.clone()).collect()
    }
}
