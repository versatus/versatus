use hbbft::crypto::{SecretKeyShare, SIG_SIZE};
use hex::FromHexError;
use primitives::Epoch;
use ritelinked::LinkedHashSet;
use serde::{Deserialize, Serialize};
use utils::hash_data;
use vrrb_core::{
    claim::Claim,
    txn::{TransactionDigest, Txn},
};

use crate::{BlockHash, ClaimList, ConvergenceBlock, RefHash, TxnList};

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
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
        secret_key: SecretKeyShare,
    ) -> ProposalBlock {
        let hashable_txns: Vec<(String, Txn)> = {
            txns.clone()
                .iter()
                .map(|(k, v)| (k.digest_string(), v.clone()))
                .collect()
        };

        let payload = hash_data!(round, epoch, hashable_txns, claims, from);

        let signature = hex::encode(secret_key.sign(payload).to_bytes().to_vec());

        let hash = hex::encode(hash_data!(
            round,
            epoch,
            hashable_txns,
            claims,
            from,
            signature
        ));

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

    pub fn decode_signature_share(&self) -> Result<[u8; 96], FromHexError> {
        let byte_vec = hex::decode(&self.signature)?;

        if byte_vec.len() != SIG_SIZE {
            return Err(FromHexError::InvalidStringLength);
        }

        let mut byte_array: [u8; 96] = [0u8; 96];
        (0..SIG_SIZE).into_iter().for_each(|i| {
            byte_array[i] = byte_vec[i];
        });

        return Ok(byte_array);
    }

    pub(crate) fn get_hashable_txns(&self) -> Vec<(String, Txn)> {
        self.txns
            .clone()
            .iter()
            .map(|(k, v)| (k.digest_string(), v.clone()))
            .collect()
    }

    pub fn remove_confirmed_txs(&mut self, prev_blocks: Vec<ConvergenceBlock>) {
        let sets: Vec<LinkedHashSet<&TransactionDigest>> =
            { prev_blocks.iter().map(|block| block.txn_id_set()).collect() };

        let prev_block_set: LinkedHashSet<&TransactionDigest> =
            { sets.into_iter().flatten().collect() };

        let curr_txns = self.txns.clone();

        let curr_set: LinkedHashSet<&TransactionDigest> =
            { curr_txns.iter().map(|(id, _)| id).collect() };

        let prev_confirmed: LinkedHashSet<TransactionDigest> = {
            let intersection = curr_set.intersection(&prev_block_set);
            intersection
                .into_iter()
                .map(|id| id.to_owned().to_owned())
                .collect()
        };

        self.txns.retain(|id, _| prev_confirmed.contains(id));
    }

    pub fn txn_id_set(&self) -> LinkedHashSet<TransactionDigest> {
        self.txns.iter().map(|(id, _)| id.clone()).collect()
    }
}
