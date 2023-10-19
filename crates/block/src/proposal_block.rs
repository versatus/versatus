use crate::{BlockHash, ClaimList, ConvergenceBlock, QuorumCertifiedTxnList, RefHash};
use hex::FromHexError;
use primitives::{Epoch, Signature};
use ritelinked::LinkedHashSet;
use serde::{Deserialize, Serialize};
use signer::engine::SignerEngine;
use utils::hash_data;
use vrrb_core::claim::Claim;
use vrrb_core::transactions::{TransactionDigest, TransactionKind};

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
#[repr(C)]
pub struct ProposalBlock {
    pub ref_block: RefHash,
    pub round: u128,
    pub epoch: Epoch,
    pub txns: QuorumCertifiedTxnList,
    pub claims: ClaimList,
    pub from: Claim,
    pub hash: BlockHash,
    pub signature: Option<Signature>,
}

impl ProposalBlock {
    /// The `build` function takes in various inputs, and builds
    /// `ProposalBlock`that consist of confirmed transactions validated by
    /// harvester
    ///
    /// Arguments:
    ///
    /// * `ref_block`: The hash of the previous block in the blockchain that
    ///   this new block is being
    /// added to.
    /// * `round`: The round parameter is of type u128 and represents the round
    ///   number of the proposal
    /// block being built.
    /// * `epoch`: time unit into the network
    /// * `txns`: `txns` is a list of quorum certified transactions.
    /// * `claims`: `claims` is a list of claims made by validators in the
    ///   network. It is used as one of
    /// the inputs to calculate the hash of the block being proposed.
    /// * `from`: The `from` parameter is of type `Claim` and represents the
    ///   claim of the harvester who is
    /// proposing the block. It is used to sign the block proposal and ensure
    /// that only the current harvester who has the private key
    /// corresponding to the public key in the claim can propose the block.
    /// * `secret_key`: The `secret_key` parameter is a reference to a `MinerSk`
    ///   struct, which likely
    /// contains the secret key of the miner used to sign the proposal block.
    ///
    /// Returns:
    ///
    /// a `ProposalBlock` object.
    pub fn build(
        ref_block: RefHash,
        round: u128,
        epoch: Epoch,
        txns: QuorumCertifiedTxnList,
        claims: ClaimList,
        from: Claim,
        mut sig_engine: SignerEngine,
    ) -> ProposalBlock {
        let hashable_txns: Vec<(String, TransactionKind)> = {
            txns.iter()
                .map(|(k, v)| (k.digest_string(), v.clone()))
                .collect()
        };
        let payload = hash_data!(round, epoch, hashable_txns, claims, from);
        let signature = if let Ok(signature) = sig_engine.sign(payload) {
            Some(signature)
        } else {
            None
        };

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

    #[deprecated]
    pub fn decode_signature_share(&self) -> Result<[u8; 96], FromHexError> {
        //        let byte_vec = hex::decode(&self.signature)?;

        //        if byte_vec.len() != SIG_SIZE {
        //            return Err(FromHexError::InvalidStringLength);
        //        }

        //        let mut byte_array: [u8; 96] = [0u8; 96];
        //        (0..SIG_SIZE).for_each(|i| {
        //            byte_array[i] = byte_vec[i];
        //        });

        //        Ok(byte_array)
        Ok([0u8; 96])
    }

    /// This function returns a vector of tuples containing the digest string
    /// and a clone of each QuorumCertifiedTxn in the original vector of
    /// transactions.
    ///
    /// Returns:
    ///
    /// A vector of tuples, where each tuple contains a string representing the
    /// digest of a transaction and a clone of the corresponding
    /// QuorumCertifiedTxn object from the original vector of transactions.
    pub(crate) fn get_hashable_txns(&self) -> Vec<(String, TransactionKind)> {
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
