use primitives::{types::SecretKey as SecretKeyBytes, Epoch};
use ritelinked::LinkedHashSet;
use secp256k1::{
    hashes::{sha256 as s256, Hash},
    Message,
};
use serde::{Deserialize, Serialize};
use sha256::digest;
use utils::{create_payload, hash_data};
use vrrb_core::claim::Claim;

use crate::{BlockHash, ClaimList, ConvergenceBlock, RefHash, TxnId, TxnList};

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
        let hash = hash_data!(round, epoch, txns, claims, from, signature);

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
        let sets: Vec<LinkedHashSet<&TxnId>> =
            { prev_blocks.iter().map(|block| block.txn_id_set()).collect() };

        let prev_block_set: LinkedHashSet<&TxnId> = { sets.into_iter().flatten().collect() };

        let curr_txns = self.txns.clone();

        let curr_set: LinkedHashSet<&TxnId> = { curr_txns.iter().map(|(id, _)| id).collect() };

        let prev_confirmed: LinkedHashSet<TxnId> = {
            let intersection = curr_set.intersection(&prev_block_set);
            intersection.into_iter().map(|id| id.to_string()).collect()
        };

        self.txns.retain(|id, _| prev_confirmed.contains(id));
    }

    pub fn txn_id_set(&self) -> LinkedHashSet<TxnId> {
        self.txns.iter().map(|(id, _)| id.clone()).collect()
    }
}
