pub mod block;
pub mod genesis;
pub mod header;
pub mod invalid;
pub use crate::block::*;

pub(crate) mod helpers {
    #![allow(unused)]
    use primitives::types::{PublicKey, SecretKey};
    use sha256::digest;
    use utils::{create_payload, hash_data, timestamp};
    use uuid::Uuid;
    use vrrb_core::{
        claim::Claim,
        keypair::KeyPair,
        txn::{NewTxnArgs, Txn},
    };

    use crate::{GenesisBlock, ProposalBlock};

    type Address = String;

    pub(crate) fn create_keypair() -> (SecretKey, PublicKey) {
        let kp = KeyPair::random();
        kp.miner_kp
    }

    pub(crate) fn create_address(pubkey: &PublicKey) -> Address {
        hash_data!(pubkey.to_string())
    }

    pub(crate) fn create_claim(pk: &PublicKey, addr: String, nonce: u128) -> Claim {
        Claim::new(pk.to_string(), addr, nonce)
    }

    pub(crate) fn mine_genesis() -> Option<GenesisBlock> {
        let (sk, pk) = create_keypair();
        let addr = create_address(&pk);
        let claim = create_claim(&pk, addr, 1);
        let claim_list = {
            vec![(claim.hash.clone(), claim.clone())]
                .iter()
                .cloned()
                .collect()
        };

        GenesisBlock::mine_genesis(claim, sk, claim_list)
    }

    pub(crate) fn create_txns(n: usize) -> impl Iterator<Item = (String, Txn)> {
        (0..n)
            .map(|n| {
                let (sk, pk) = create_keypair();
                let raddr = "0x192abcdef01234567890".to_string();
                let saddr = create_address(&pk);
                let amount = (n.pow(2)) as u128;
                let nonce = 1u128;
                let token = Some("VRRB".to_string());
                let mut txn: Txn = Txn {
                    txn_id: Uuid::new_v4(),
                    timestamp: timestamp!(),
                    sender_address: saddr,
                    sender_public_key: pk.serialize().to_vec(),
                    receiver_address: raddr,
                    token,
                    amount,
                    payload: None,
                    signature: None,
                    validators: None,
                    nonce: n.clone() as u128,
                };

                txn.sign(&sk);
                let txn_hash = hash_data!(&txn);
                (txn_hash, txn)
            })
            .into_iter()
    }

    pub(crate) fn create_claims(n: usize) -> impl Iterator<Item = (String, Claim)> {
        (0..n)
            .map(|_| {
                let (_, pk) = create_keypair();
                let addr = create_address(&pk);
                let claim = create_claim(&pk, addr.clone(), 1);
                (claim.hash.clone(), claim.clone())
            })
            .into_iter()
    }

    pub(crate) fn build_proposal_block(
        genesis: GenesisBlock,
        n_tx: usize,
        n_claims: usize,
        round: u128,
        epoch: u128,
    ) -> ProposalBlock {
        let (sk, pk) = create_keypair();
        let round = 0;
        let epoch = 0;
        let txns = create_txns(n_tx).collect();
        let claims = create_claims(n_claims).collect();
        let hclaim = create_claim(&pk, create_address(&pk), 1);
        let ref_hash = genesis.hash;

        ProposalBlock::build(ref_hash, round, epoch, txns, claims, hclaim, sk)
    }
}

#[cfg(test)]
mod tests {
    #![allow(unused_imports)]
    use std::{collections::HashMap, str::FromStr, time::UNIX_EPOCH};

    use rand::Rng;
    use reward::reward::Reward;
    use ritelinked::LinkedHashMap;
    use secp256k1::{
        ecdsa::Signature,
        hashes::{sha256 as s256, Hash},
        Message,
        PublicKey,
    };
    use vrrb_core::{
        claim::Claim,
        keypair::KeyPair,
        txn::{NewTxnArgs, Txn},
    };

    use crate::{
        header::BlockHeader,
        helpers::*,
        Block,
        Conflict,
        ConvergenceBlock,
        GenesisBlock,
        MineArgs,
        ProposalBlock,
    };

    #[test]
    fn test_create_genesis_block() {
        let genesis = mine_genesis();
        assert!(genesis.is_some());
    }

    #[test]
    fn test_create_proposal_block() {
        let genesis = mine_genesis().unwrap();
        let proposal = build_proposal_block(genesis, 30, 10, 0, 0);

        let payload = utils::create_payload!(
            proposal.round,
            proposal.epoch,
            proposal.txns,
            proposal.claims,
            proposal.from
        );
        let h_pk = proposal.from.public_key;
        let h_pk = PublicKey::from_str(&h_pk).unwrap();
        let sig = proposal.signature;
        let sig = Signature::from_str(&sig).unwrap();
        let verify = sig.verify(&payload, &h_pk);
        assert!(verify.is_ok())
    }

    #[ignore]
    #[test]
    fn test_create_convergence_block_no_conflicts() {
        todo!()
    }

    #[ignore]
    #[test]
    fn test_create_convergence_block_conflicts() {
        todo!()
    }

    #[ignore]
    #[test]
    fn test_resolve_conflicts_valid() {
        todo!()
    }

    #[ignore]
    #[test]
    fn test_epoch_change() {
        todo!()
    }

    #[ignore]
    #[test]
    fn test_utility_adjustment() {
        todo!()
    }
}
