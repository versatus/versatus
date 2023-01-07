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
    use bulldag::{
        edge::Edge, 
        graph::BullDag, 
        vertex::{
            Direction, 
            Edges, 
            Vertex
        }
    };
    use vrrb_core::{
        claim::Claim,
        keypair::KeyPair,
        txn::{NewTxnArgs, Txn},
    };

    use crate::{
        GenesisBlock, 
        ProposalBlock, 
        ConvergenceBlock, 
        Block, 
        EPOCH_BLOCK,
        MineArgs,
        InnerBlock
    };

    type Address = String;

    pub(crate) fn create_keypair() -> (SecretKey, PublicKey) {
        let kp = KeyPair::random();
        kp.miner_kp
    }

    pub(crate) fn create_address(pubkey: &PublicKey) -> Address {
        hash_data!(pubkey.to_string())
    }

    pub(crate) fn create_claim(
        pk: &PublicKey, 
        addr: &String, 
        nonce: &u128
    ) -> Claim {
        Claim::new(pk.to_string(), addr.to_string(), nonce.to_owned())
    }

    pub(crate) fn mine_genesis() -> Option<GenesisBlock> {
        let (sk, pk) = create_keypair();
        let addr = create_address(&pk);
        let claim = create_claim(&pk, &addr, &1);
        let claim_list = {
            vec![(claim.hash.clone(), claim.clone())]
                .iter()
                .cloned()
                .collect()
        };

        GenesisBlock::mine_genesis(claim, sk, claim_list)
    }

    pub(crate) fn create_txns(
        n: usize
    ) -> impl Iterator<Item = (String, Txn)> {
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

    pub(crate) fn create_claims(
        n: usize
    ) -> impl Iterator<Item = (String, Claim)> {
        (0..n)
            .map(|_| {
                let (_, pk) = create_keypair();
                let addr = create_address(&pk);
                let claim = create_claim(&pk, &addr, &1);
                (claim.hash.clone(), claim.clone())
            })
            .into_iter()
    }

    pub(crate) fn build_proposal_block(
        ref_hash: &String,
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
        let hclaim = create_claim(&pk, &create_address(&pk), &1);

        ProposalBlock::build(
            ref_hash.clone(), 
            round, 
            epoch, 
            txns, 
            claims, 
            hclaim, 
            sk
        )
    }

    pub(crate) fn mine_convergence_block(
        proposals: &Vec<ProposalBlock>,
        chain: &BullDag<Block, String>,
        last_block: Block,
    ) -> Option<ConvergenceBlock> {

        let (msk, mpk) = create_keypair();
        let maddr = create_address(&mpk);
        let miner_claim = create_claim(&mpk, &maddr, &1); 
        let txns = create_txns(30).collect();
        let claims = create_claims(5).collect();
        let claim_list_hash = Some(hash_data!(claims));
        
        let mut reward = {
            match last_block {
                Block::Convergence { ref block } => {
                    block.header.next_block_reward.clone()
                }
                Block::Genesis { ref block } => {
                    block.header.next_block_reward.clone()
                }
                _ => return None
            }
        };
        
        let epoch = {
            match last_block {
                Block::Convergence { ref block } => {
                    if block.header.block_height % EPOCH_BLOCK as u128 == 0 {
                        block.header.epoch + 1
                    } else {
                        block.header.epoch
                    }
                }
                Block::Genesis { ref block } => {
                    0
                }
                _ => return None
            }
        };

        let round = {
            match last_block {
                Block::Convergence { ref block } => {
                    block.header.round + 1
                }
                Block::Genesis { .. } => {
                    1
                }
                _ => return None
            }
        };
        
        let mine_args = MineArgs {
            claim: miner_claim,
            last_block: last_block.clone(),
            txns,
            claims,
            claim_list_hash,
            reward: &mut reward,
            abandoned_claim: None,
            secret_key: msk,
            epoch,
            round
        };

        ConvergenceBlock::mine(
            mine_args,
            proposals,
            chain
        ) 
    }
}

#[cfg(test)]
mod tests {
    #![allow(unused_imports)]
    use std::{collections::HashMap, str::FromStr, time::UNIX_EPOCH};
    use bulldag::{
        graph::BullDag,
        vertex::{
            Vertex,
            Edges,
            Direction
        }
    };

    use rand::Rng;
    use reward::reward::Reward;
    use ritelinked::LinkedHashMap;
    use secp256k1::{
        ecdsa::Signature,
        hashes::{sha256 as s256, Hash},
        Message,
        PublicKey,
    };

    use utils::create_payload;
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
        let proposal = build_proposal_block(&genesis.hash, 30, 10, 0, 0);

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

    #[test]
    fn test_create_convergence_block_no_conflicts() {
        let genesis = mine_genesis();
        if let Some(gblock) = genesis {
            let ref_hash = gblock.hash.clone();
            let round = gblock.header.round.clone() + 1;
            let epoch = gblock.header.epoch.clone();

            let prop1 = build_proposal_block(
                &ref_hash,
                30,
                10,
                round,
                epoch
            ); 

            let prop2 = build_proposal_block(
                &ref_hash,
                40,
                5,
                round,
                epoch
            );
            
            let proposals = vec![prop1.clone(), prop2.clone()];
            
            let mut chain: BullDag<Block, String> = BullDag::new();

            let gvtx = Vertex::new(
                Block::Genesis { block: gblock.clone() }, 
                gblock.hash.clone() 
            ); 

            let p1vtx = Vertex::new(
                Block::Proposal { block: prop1.clone() },
                prop1.hash.clone()
            );

            let p2vtx = Vertex::new(
                Block::Proposal { block: prop2.clone() },
                prop2.hash.clone()
            );

            let edges = vec![
                (&gvtx, &p1vtx),
                (&gvtx, &p2vtx)
            ];

            chain.extend_from_edges(
                edges
            );

            let c_block = mine_convergence_block(
                &proposals,
                &chain,
                Block::Genesis { block: gblock }
            );

            if let Some(cb) = c_block {
                let sig = cb.header.miner_signature;
                let sig = Signature::from_str(&sig).unwrap();

                let payload = create_payload!(
                    cb.header.ref_hashes,
                    cb.header.round,
                    cb.header.epoch,
                    cb.header.block_seed,
                    cb.header.next_block_seed,
                    cb.header.block_height,
                    cb.header.timestamp,
                    cb.header.txn_hash,
                    cb.header.miner_claim,
                    cb.header.claim_list_hash,
                    cb.header.block_reward,
                    cb.header.next_block_reward
                );

                let mpk = cb.header.miner_claim.public_key;
                let mpk = PublicKey::from_str(&mpk).unwrap();

                let verify = sig.verify(&payload, &mpk);

                assert!(verify.is_ok())
            }
        } else {
            panic!("A ConvergenceBlock should be produced")
        }
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
