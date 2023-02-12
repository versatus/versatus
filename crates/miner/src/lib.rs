pub mod miner;
pub mod result;
pub use crate::miner::*;
// mod miner_v1;

/// Legacy miner implementation
#[deprecated(note = "use v2 instead")]
pub mod v1 {
    // pub use crate::miner_v1::*;
}

pub mod v2 {
    pub use crate::miner::*;
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, str::FromStr};

    use block::{header::BlockHeader, Block, ConvergenceBlock};
    use bulldag::{graph::BullDag, vertex::Vertex};
    use primitives::{PublicKey, SecretKey, Signature};
    use reward::reward::Reward;
    use ritelinked::{LinkedHashMap, LinkedHashSet};
    use secp256k1::{
        hashes::{sha256 as s256, Hash},
        rand,
        Message,
    };
    use sha256::digest;
    use utils::{create_payload, hash_data};
    use vrrb_core::{
        keypair::{Keypair, SecretKeys},
        txn::Txn,
    };

    use super::test_helpers::create_txns;
    use crate::{
        test_helpers::{
            build_proposal_block,
            create_claim,
            create_keypair,
            create_miner,
            mine_convergence_block,
            mine_convergence_block_epoch_change,
            mine_genesis,
        },
        MineArgs,
        Miner,
        MinerConfig,
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

        let payload = create_payload!(
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

            let prop1 = build_proposal_block(&ref_hash, 30, 10, round, epoch);

            let prop2 = build_proposal_block(&ref_hash, 40, 5, round, epoch);

            let proposals = vec![prop1.clone(), prop2.clone()];

            let mut chain: BullDag<Block, String> = BullDag::new();

            let gvtx = Vertex::new(
                Block::Genesis {
                    block: gblock.clone(),
                },
                gblock.hash.clone(),
            );

            let p1vtx = Vertex::new(
                Block::Proposal {
                    block: prop1.clone(),
                },
                prop1.hash.clone(),
            );

            let p2vtx = Vertex::new(
                Block::Proposal {
                    block: prop2.clone(),
                },
                prop2.hash.clone(),
            );

            let edges = vec![(&gvtx, &p1vtx), (&gvtx, &p2vtx)];

            chain.extend_from_edges(edges);

            let c_block =
                mine_convergence_block(&proposals, &chain, Block::Genesis { block: gblock });

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

    #[test]
    fn test_resolve_conflicts_curr_round() {
        // Create a large transaction map
        let genesis = mine_genesis();
        if let Some(gblock) = genesis {
            let ref_hash = gblock.hash.clone();
            let round = gblock.header.round + 1;
            let epoch = gblock.header.epoch;

            let mut prop1 = build_proposal_block(&ref_hash, 30, 10, round, epoch);

            let mut prop2 = build_proposal_block(&ref_hash, 40, 5, round, epoch);

            let txns: HashMap<String, Txn> = create_txns(5).collect();
            prop1.txns.extend(txns.clone());
            prop2.txns.extend(txns.clone());

            let proposals = vec![prop1.clone(), prop2.clone()];

            let mut chain: BullDag<Block, String> = BullDag::new();

            let gvtx = Vertex::new(
                Block::Genesis {
                    block: gblock.clone(),
                },
                gblock.hash.clone(),
            );

            let p1vtx = Vertex::new(
                Block::Proposal {
                    block: prop1.clone(),
                },
                prop1.hash.clone(),
            );

            let p2vtx = Vertex::new(
                Block::Proposal {
                    block: prop2.clone(),
                },
                prop2.hash.clone(),
            );

            let edges = vec![(&gvtx, &p1vtx), (&gvtx, &p2vtx)];

            chain.extend_from_edges(edges);

            let c_block =
                mine_convergence_block(&proposals, &chain, Block::Genesis { block: gblock });

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

                assert!(verify.is_ok());

                let total_w_duplicates = { prop1.txns.keys().len() + prop2.txns.keys().len() };

                assert!(total_w_duplicates > cb.txns.len());

                // Get the winner of the PoC election between proposer 1 and 2
                let mut proposer_ps = vec![
                    (
                        prop1.hash,
                        prop1.from.get_pointer(cb.header.block_seed as u128),
                    ),
                    (
                        prop2.hash,
                        prop2.from.get_pointer(cb.header.block_seed as u128),
                    ),
                ];

                // The first will be the winner
                proposer_ps.sort_unstable_by(|(_, a_pointer), (_, b_pointer)| {
                    match (a_pointer, b_pointer) {
                        (Some(x), Some(y)) => x.cmp(y),
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, None) => std::cmp::Ordering::Equal,
                    }
                });
                let winner = proposer_ps[0].0.clone();
                let mut resolved_conflicts = cb.txns.clone();
                resolved_conflicts.retain(|_, set| {
                    let conflicts = txns.keys().cloned().collect();
                    let intersection: LinkedHashSet<&String> =
                        set.intersection(&conflicts).collect();
                    intersection.len() > 0
                });

                let key: Vec<String> = resolved_conflicts.keys().cloned().collect();
                assert!(key.len() == 1);
                assert_eq!(key[0], winner);
            }
        } else {
            panic!("A ConvergenceBlock should be produced")
        }
    }

    #[test]
    fn test_resolve_conflicts_prev_rounds() {
        let genesis = mine_genesis();
        if let Some(gblock) = genesis {
            let ref_hash = gblock.hash.clone();
            let round = gblock.header.round.clone() + 1;
            let epoch = gblock.header.epoch.clone();

            let mut prop1 = build_proposal_block(&ref_hash, 30, 10, round, epoch);

            let mut prop2 = build_proposal_block(&ref_hash, 40, 5, round, epoch);

            let txns: HashMap<String, Txn> = create_txns(5).collect();
            prop1.txns.extend(txns.clone());

            let proposals = vec![prop1.clone(), prop2.clone()];

            let mut chain: BullDag<Block, String> = BullDag::new();

            let gvtx = Vertex::new(
                Block::Genesis {
                    block: gblock.clone(),
                },
                gblock.hash.clone(),
            );

            let p1vtx = Vertex::new(
                Block::Proposal {
                    block: prop1.clone(),
                },
                prop1.hash.clone(),
            );

            let p2vtx = Vertex::new(
                Block::Proposal {
                    block: prop2.clone(),
                },
                prop2.hash.clone(),
            );

            let edges = vec![(&gvtx, &p1vtx), (&gvtx, &p2vtx)];

            chain.extend_from_edges(edges);

            let c_block_1 = mine_convergence_block(
                &proposals,
                &chain,
                Block::Genesis {
                    block: gblock.clone(),
                },
            );

            let cb1 = c_block_1.unwrap();

            let cb1vtx = Vertex::new(Block::Convergence { block: cb1.clone() }, cb1.hash.clone());

            let edges = vec![(&p1vtx, &cb1vtx), (&p2vtx, &cb1vtx)];

            chain.extend_from_edges(edges);

            let mut prop3 = build_proposal_block(&ref_hash, 20, 10, round, epoch);

            prop3.txns.extend(txns.clone());

            let c_block_2 = mine_convergence_block(
                &vec![prop3.clone()],
                &chain,
                Block::Genesis {
                    block: gblock.clone(),
                },
            );

            let cb2 = {
                let cb = c_block_2.unwrap();
                let sig = cb.header.miner_signature.clone();
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

                let mpk = cb.header.miner_claim.public_key.clone();
                let mpk = PublicKey::from_str(&mpk).unwrap();

                let verify = sig.verify(&payload, &mpk);

                assert!(verify.is_ok());

                let total_w_duplicates = { prop3.txns.keys().len() };

                assert!(total_w_duplicates > cb.txns.len());

                cb
            };

            let p3vtx = Vertex::new(
                Block::Proposal {
                    block: prop3.clone(),
                },
                prop3.hash.clone(),
            );

            let cb2vtx = Vertex::new(Block::Convergence { block: cb2.clone() }, cb2.hash.clone());

            let edges = vec![(&gvtx, &p3vtx), (&p3vtx, &cb2vtx)];

            chain.extend_from_edges(edges);
        }
    }

    #[test]
    fn test_epoch_change() {
        let (msk1, mpk1) = create_keypair();

        let miner = create_miner();
        let addr = miner.address();
        let nonce = 1;

        let ref_hashes = vec!["abcdef".to_string()];
        let epoch = 0;
        let round = 29_999_998;
        let block_seed = 34_989_333;
        let next_block_seed = 839_999_843;
        let block_height = 29_999_998;
        let timestamp = utils::timestamp!();
        let txn_hash = "abcdef01234567890".to_string();
        let miner_claim = miner.generate_claim(nonce);
        let claim_list_hash = "01234567890abcdef".to_string();
        let mut block_reward = Reward::default();
        block_reward.current_block = block_height;
        let next_block_reward = block_reward.clone();

        let payload = create_payload!(
            ref_hashes,
            round,
            epoch,
            block_seed,
            next_block_seed,
            block_height,
            timestamp,
            txn_hash,
            miner_claim,
            claim_list_hash,
            block_reward,
            next_block_reward
        );

        let miner_signature = miner.sign_message(payload).to_string();

        let header = BlockHeader {
            ref_hashes,
            round,
            epoch,
            block_seed,
            next_block_seed,
            block_height,
            timestamp,
            txn_hash,
            miner_claim,
            claim_list_hash,
            block_reward,
            next_block_reward,
            miner_signature,
        };

        let txns = LinkedHashMap::new();
        let claims = LinkedHashMap::new();
        let block_hash = hash_data!(
            header.ref_hashes,
            header.round,
            header.epoch,
            header.block_seed,
            header.next_block_seed,
            header.block_height,
            header.timestamp,
            header.txn_hash,
            header.miner_claim,
            header.claim_list_hash,
            header.block_reward,
            header.next_block_reward,
            header.miner_signature
        );

        let cb1 = ConvergenceBlock {
            header,
            txns,
            claims,
            hash: block_hash,
            certificate: None,
        };

        let mut chain: BullDag<Block, String> = BullDag::new();

        let prop1 = build_proposal_block(&cb1.hash.clone(), 5, 5, 30_000_000, 0);

        let cb1vtx = Vertex::new(Block::Convergence { block: cb1.clone() }, cb1.hash.clone());

        let p1vtx = Vertex::new(
            Block::Proposal {
                block: prop1.clone(),
            },
            prop1.hash.clone(),
        );

        let edges = vec![(&cb1vtx, &p1vtx)];

        chain.extend_from_edges(edges);

        let cb2 = mine_convergence_block_epoch_change(
            &vec![prop1.clone()],
            &chain,
            &Block::Convergence { block: cb1.clone() },
            0,
        )
        .unwrap();

        assert_eq!(cb2.header.next_block_reward.epoch, 1);
        assert_eq!(cb2.header.next_block_reward.next_epoch_block, 60_000_000);
    }

    #[test]
    fn test_utility_adjustment() {
        let (msk1, mpk1) = create_keypair();

        let miner = create_miner();
        let addr = miner.address();

        let ref_hashes = vec!["abcdef".to_string()];
        let epoch = 0;
        let round = 29_999_998;
        let block_seed = 34_989_333;
        let next_block_seed = 839_999_843;
        let block_height = 29_999_998;
        let timestamp = utils::timestamp!();
        let txn_hash = "abcdef01234567890".to_string();

        let miner_claim = create_claim(&mpk1, &addr, 1);

        let claim_list_hash = "01234567890abcdef".to_string();

        let mut block_reward = Reward::default();
        block_reward.current_block = block_height;
        let next_block_reward = block_reward.clone();

        let payload = create_payload!(
            ref_hashes,
            round,
            epoch,
            block_seed,
            next_block_seed,
            block_height,
            timestamp,
            txn_hash,
            miner_claim,
            claim_list_hash,
            block_reward,
            next_block_reward
        );

        let miner_signature = msk1.sign_ecdsa(payload).to_string();

        let header = BlockHeader {
            ref_hashes,
            round,
            epoch,
            block_seed,
            next_block_seed,
            block_height,
            timestamp,
            txn_hash,
            miner_claim,
            claim_list_hash,
            block_reward,
            next_block_reward,
            miner_signature,
        };

        let txns = LinkedHashMap::new();
        let claims = LinkedHashMap::new();
        let block_hash = hash_data!(
            header.ref_hashes,
            header.round,
            header.epoch,
            header.block_seed,
            header.next_block_seed,
            header.block_height,
            header.timestamp,
            header.txn_hash,
            header.miner_claim,
            header.claim_list_hash,
            header.block_reward,
            header.next_block_reward,
            header.miner_signature
        );

        let cb1 = ConvergenceBlock {
            header,
            txns,
            claims,
            hash: block_hash,
            certificate: None,
        };

        let mut chain: BullDag<Block, String> = BullDag::new();

        let prop1 = build_proposal_block(&cb1.hash.clone(), 5, 5, 30_000_000, 0);
        let cb1vtx = Vertex::new(Block::Convergence { block: cb1.clone() }, cb1.hash.clone());

        let p1vtx = Vertex::new(
            Block::Proposal {
                block: prop1.clone(),
            },
            prop1.hash.clone(),
        );

        let edges = vec![(&cb1vtx, &p1vtx)];

        chain.extend_from_edges(edges);

        let cb2 = mine_convergence_block_epoch_change(
            &vec![prop1.clone()],
            &chain,
            &Block::Convergence { block: cb1.clone() },
            (4 * 30_000_000) as i128,
        )
        .unwrap();

        assert_eq!(cb2.header.next_block_reward.amount, 24);
    }
}

pub(crate) mod test_helpers {
    use block::{Block, ConvergenceBlock, GenesisBlock, ProposalBlock, EPOCH_BLOCK};
    use bulldag::graph::BullDag;
    use primitives::{
        types::{PublicKey, SecretKey},
        Address,
    };
    use sha256::digest;
    use utils::hash_data;
    use vrrb_core::{
        claim::Claim,
        keypair::KeyPair,
        txn::{NewTxnArgs, Txn},
    };

    use crate::{MineArgs, Miner, MinerConfig};

    pub(crate) fn create_miner() -> Miner {
        let (secret_key, public_key) = create_keypair();

        let address = create_address(&public_key);

        let config = MinerConfig {
            secret_key,
            public_key,
            address,
        };

        Miner::new(config)
    }

    pub(crate) fn create_keypair() -> (SecretKey, PublicKey) {
        let kp = KeyPair::random();
        kp.miner_kp
    }

    pub(crate) fn create_address(pubkey: &PublicKey) -> Address {
        hash_data!(pubkey.to_string())
    }

    pub(crate) fn create_claim(pk: &PublicKey, addr: &str, nonce: u128) -> Claim {
        Claim::new(pk.to_string(), addr.to_string(), nonce)
    }

    pub(crate) fn mine_genesis() -> Option<GenesisBlock> {
        let (sk, pk) = create_keypair();
        let addr = create_address(&pk);
        let miner = create_miner();

        let claim = miner.generate_claim(1);

        let claim_list = {
            vec![(claim.hash.clone(), claim.clone())]
                .iter()
                .cloned()
                .collect()
        };

        miner.mine_genesis_block(claim_list, 1)
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
                let txn_args = NewTxnArgs {
                    sender_address: saddr,
                    sender_public_key: pk.serialize().to_vec(),
                    receiver_address: raddr,
                    token,
                    amount,
                    payload: None,
                    signature: vec![],
                    validators: None,
                    nonce: n.clone() as u128,
                };

                let mut txn = Txn::new(txn_args);

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
                let claim = create_claim(&pk, &addr, 1);
                (claim.hash.clone(), claim)
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
        let txns = create_txns(n_tx).collect();
        let nonce = 1;

        let claims = create_claims(n_claims).collect();
        let hclaim = create_claim(&pk, &create_address(&pk), 1);

        let miner = create_miner();

        miner.build_proposal_block(ref_hash.clone(), round, epoch, txns, claims, nonce)
    }

    pub(crate) fn mine_convergence_block(
        proposals: &Vec<ProposalBlock>,
        chain: &BullDag<Block, String>,
        last_block: Block,
    ) -> Option<ConvergenceBlock> {
        let (msk, mpk) = create_keypair();
        let maddr = create_address(&mpk);
        let miner_claim = create_claim(&mpk, &maddr, 1);
        let txns = create_txns(30).collect();
        let claims = create_claims(5).collect();
        let claim_list_hash = Some(hash_data!(claims));

        let miner = create_miner();
        let maddr = miner.address();

        let mut reward = {
            match last_block {
                Block::Convergence { ref block } => block.header.next_block_reward.clone(),
                Block::Genesis { ref block } => block.header.next_block_reward.clone(),
                _ => return None,
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
                },
                Block::Genesis { ref block } => 0,
                _ => return None,
            }
        };

        let round = {
            match last_block {
                Block::Convergence { ref block } => block.header.round + 1,
                Block::Genesis { .. } => 1,
                _ => return None,
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
            round,
            next_epoch_adjustment: 0,
        };

        let miner = create_miner();
        miner.mine_convergence_block(mine_args, proposals, chain)
        // ConvergenceBlock::mine(mine_args, proposals, chain)
    }

    pub(crate) fn mine_convergence_block_epoch_change(
        proposals: &Vec<ProposalBlock>,
        chain: &BullDag<Block, String>,
        last_block: &Block,
        next_epoch_adjustment: i128,
    ) -> Option<ConvergenceBlock> {
        let (msk, mpk) = create_keypair();
        let maddr = create_address(&mpk);
        let miner_claim = create_claim(&mpk, &maddr, 1);

        let txns = create_txns(30).collect();
        let claims = create_claims(5).collect();
        let claim_list_hash = Some(hash_data!(claims));

        let mut reward = {
            match last_block {
                Block::Convergence { ref block } => block.header.next_block_reward.clone(),
                Block::Genesis { ref block } => block.header.next_block_reward.clone(),
                _ => return None,
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
                },
                Block::Genesis { ref block } => 0,
                _ => return None,
            }
        };

        let round = {
            match last_block {
                Block::Convergence { ref block } => block.header.round + 1,
                Block::Genesis { .. } => 1,
                _ => return None,
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
            round,
            next_epoch_adjustment,
        };

        let miner = create_miner();

        miner.mine_convergence_block(mine_args, proposals, chain)
    }
}

// #[cfg(test)]
// mod tests {
//     use std::{collections::HashMap, str::FromStr, time::UNIX_EPOCH};
//
//     use bulldag::{
//         graph::BullDag,
//         vertex::{Direction, Edges, Vertex},
//     };
//     use rand::Rng;
//     use reward::reward::Reward;
//     use ritelinked::{LinkedHashMap, LinkedHashSet};
//     use secp256k1::{
//         ecdsa::Signature,
//         hashes::{sha256 as s256, Hash},
//         Message, PublicKey,
//     };
//     use sha256::digest;
//     use utils::{create_payload, hash_data};
//     use vrrb_core::{
//         claim::Claim,
//         keypair::KeyPair,
//         txn::{NewTxnArgs, Txn},
//     };
//
//     use crate::{
//         header::BlockHeader, helpers::*, Block, Conflict, ConvergenceBlock,
// GenesisBlock, MineArgs,         ProposalBlock,
//     };
//
//
//
//
// }
//
// pub(crate) mod helpers {
//     use bulldag::graph::BullDag;
//     use primitives::types::{PublicKey, SecretKey};
//     use sha256::digest;
//     use utils::hash_data;
//     use vrrb_core::{
//         claim::Claim,
//         keypair::KeyPair,
//         txn::{NewTxnArgs, Txn},
//     };
//
//     use crate::{Block, ConvergenceBlock, GenesisBlock, MineArgs,
// ProposalBlock, EPOCH_BLOCK};
//
//     type Address = String;
//
//     pub(crate) fn create_keypair() -> (SecretKey, PublicKey) {
//         let kp = KeyPair::random();
//         kp.miner_kp
//     }
//
//     pub(crate) fn create_address(pubkey: &PublicKey) -> Address {
//         hash_data!(pubkey.to_string())
//     }
//
//     pub(crate) fn create_claim(pk: &PublicKey, addr: &String, nonce: &u128)
// -> Claim {         Claim::new(pk.to_string(), addr.to_string(),
// nonce.to_owned())     }
//
//     pub(crate) fn mine_genesis() -> Option<GenesisBlock> {
//         let (sk, pk) = create_keypair();
//         let addr = create_address(&pk);
//         let claim = create_claim(&pk, &addr, &1);
//         let claim_list = {
//             vec![(claim.hash.clone(), claim.clone())]
//                 .iter()
//                 .cloned()
//                 .collect()
//         };
//
//         GenesisBlock::mine_genesis(claim, sk, claim_list)
//     }
//
//     pub(crate) fn create_txns(n: usize) -> impl Iterator<Item = (String,
// Txn)> {         (0..n)
//             .map(|n| {
//                 let (sk, pk) = create_keypair();
//                 let raddr = "0x192abcdef01234567890".to_string();
//                 let saddr = create_address(&pk);
//                 let amount = (n.pow(2)) as u128;
//                 let nonce = 1u128;
//                 let token = Some("VRRB".to_string());
//                 let txn_args = NewTxnArgs {
//                     sender_address: saddr,
//                     sender_public_key: pk.serialize().to_vec(),
//                     receiver_address: raddr,
//                     token,
//                     amount,
//                     payload: None,
//                     signature: vec![],
//                     validators: None,
//                     nonce: n.clone() as u128,
//                 };
//
//                 let mut txn = Txn::new(txn_args);
//
//                 txn.sign(&sk);
//
//                 let txn_hash = hash_data!(&txn);
//                 (txn_hash, txn)
//             })
//             .into_iter()
//     }
//
//     pub(crate) fn create_claims(n: usize) -> impl Iterator<Item = (String,
// Claim)> {         (0..n)
//             .map(|_| {
//                 let (_, pk) = create_keypair();
//                 let addr = create_address(&pk);
//                 let claim = create_claim(&pk, &addr, &1);
//                 (claim.hash.clone(), claim.clone())
//             })
//             .into_iter()
//     }
//
//     pub(crate) fn build_proposal_block(
//         ref_hash: &String,
//         n_tx: usize,
//         n_claims: usize,
//         round: u128,
//         epoch: u128,
//     ) -> ProposalBlock {
//         let (sk, pk) = create_keypair();
//         let txns = create_txns(n_tx).collect();
//         let claims = create_claims(n_claims).collect();
//         let hclaim = create_claim(&pk, &create_address(&pk), &1);
//
//         ProposalBlock::build(ref_hash.clone(), round, epoch, txns, claims,
// hclaim, sk)     }
//
//     pub(crate) fn mine_convergence_block(
//         proposals: &Vec<ProposalBlock>,
//         chain: &BullDag<Block, String>,
//         last_block: Block,
//     ) -> Option<ConvergenceBlock> {
//         let (msk, mpk) = create_keypair();
//         let maddr = create_address(&mpk);
//         let miner_claim = create_claim(&mpk, &maddr, &1);
//         let txns = create_txns(30).collect();
//         let claims = create_claims(5).collect();
//         let claim_list_hash = Some(hash_data!(claims));
//
//         let mut reward = {
//             match last_block {
//                 Block::Convergence { ref block } =>
// block.header.next_block_reward.clone(),                 Block::Genesis { ref
// block } => block.header.next_block_reward.clone(),                 _ =>
// return None,             }
//         };
//
//         let epoch = {
//             match last_block {
//                 Block::Convergence { ref block } => {
//                     if block.header.block_height % EPOCH_BLOCK as u128 == 0 {
//                         block.header.epoch + 1
//                     } else {
//                         block.header.epoch
//                     }
//                 },
//                 Block::Genesis { ref block } => 0,
//                 _ => return None,
//             }
//         };
//
//         let round = {
//             match last_block {
//                 Block::Convergence { ref block } => block.header.round + 1,
//                 Block::Genesis { .. } => 1,
//                 _ => return None,
//             }
//         };
//
//         let mine_args = MineArgs {
//             claim: miner_claim,
//             last_block: last_block.clone(),
//             txns,
//             claims,
//             claim_list_hash,
//             reward: &mut reward,
//             abandoned_claim: None,
//             secret_key: msk,
//             epoch,
//             round,
//             next_epoch_adjustment: 0,
//         };
//
//         ConvergenceBlock::mine(mine_args, proposals, chain)
//     }
//
//     pub(crate) fn mine_convergence_block_epoch_change(
//         proposals: &Vec<ProposalBlock>,
//         chain: &BullDag<Block, String>,
//         last_block: &Block,
//         next_epoch_adjustment: i128,
//     ) -> Option<ConvergenceBlock> {
//         let (msk, mpk) = create_keypair();
//         let maddr = create_address(&mpk);
//         let miner_claim = create_claim(&mpk, &maddr, &1);
//         let txns = create_txns(30).collect();
//         let claims = create_claims(5).collect();
//         let claim_list_hash = Some(hash_data!(claims));
//
//         let mut reward = {
//             match last_block {
//                 Block::Convergence { ref block } =>
// block.header.next_block_reward.clone(),                 Block::Genesis { ref
// block } => block.header.next_block_reward.clone(),                 _ =>
// return None,             }
//         };
//
//         let epoch = {
//             match last_block {
//                 Block::Convergence { ref block } => {
//                     if block.header.block_height % EPOCH_BLOCK as u128 == 0 {
//                         block.header.epoch + 1
//                     } else {
//                         block.header.epoch
//                     }
//                 },
//                 Block::Genesis { ref block } => 0,
//                 _ => return None,
//             }
//         };
//
//         let round = {
//             match last_block {
//                 Block::Convergence { ref block } => block.header.round + 1,
//                 Block::Genesis { .. } => 1,
//                 _ => return None,
//             }
//         };
//
//         let mine_args = MineArgs {
//             claim: miner_claim,
//             last_block: last_block.clone(),
//             txns,
//             claims,
//             claim_list_hash,
//             reward: &mut reward,
//             abandoned_claim: None,
//             secret_key: msk,
//             epoch,
//             round,
//             next_epoch_adjustment,
//         };
//
//         ConvergenceBlock::mine(mine_args, proposals, chain)
//     }
// }
//
// #[cfg(test)]
// mod tests {
//     #![allow(unused_imports)]
//     use std::{collections::HashMap, str::FromStr, time::UNIX_EPOCH};
//
//     use bulldag::{
//         graph::BullDag,
//         vertex::{Direction, Edges, Vertex},
//     };
//     use rand::Rng;
//     use reward::reward::Reward;
//     use ritelinked::{LinkedHashMap, LinkedHashSet};
//     use secp256k1::{
//         ecdsa::Signature,
//         hashes::{sha256 as s256, Hash},
//         Message, PublicKey,
//     };
//     use sha256::digest;
//     use utils::{create_payload, hash_data};
//     use vrrb_core::{
//         claim::Claim,
//         keypair::KeyPair,
//         txn::{NewTxnArgs, Txn},
//     };
//
//     use crate::{
//         header::BlockHeader, helpers::*, Block, Conflict, ConvergenceBlock,
// GenesisBlock, MineArgs,         ProposalBlock,
//     };
//
//     #[test]
//     fn test_create_genesis_block() {
//         let genesis = mine_genesis();
//         assert!(genesis.is_some());
//     }
//
//     #[test]
//     fn test_create_proposal_block() {
//         let genesis = mine_genesis().unwrap();
//         let proposal = build_proposal_block(&genesis.hash, 30, 10, 0, 0);
//
//         let payload = utils::create_payload!(
//             proposal.round,
//             proposal.epoch,
//             proposal.txns,
//             proposal.claims,
//             proposal.from
//         );
//         let h_pk = proposal.from.public_key;
//         let h_pk = PublicKey::from_str(&h_pk).unwrap();
//         let sig = proposal.signature;
//         let sig = Signature::from_str(&sig).unwrap();
//         let verify = sig.verify(&payload, &h_pk);
//         assert!(verify.is_ok())
//     }
//
//     #[test]
//     fn test_create_convergence_block_no_conflicts() {
//         let genesis = mine_genesis();
//         if let Some(gblock) = genesis {
//             let ref_hash = gblock.hash.clone();
//             let round = gblock.header.round.clone() + 1;
//             let epoch = gblock.header.epoch.clone();
//
//             let prop1 = build_proposal_block(&ref_hash, 30, 10, round,
// epoch);
//
//             let prop2 = build_proposal_block(&ref_hash, 40, 5, round, epoch);
//
//             let proposals = vec![prop1.clone(), prop2.clone()];
//
//             let mut chain: BullDag<Block, String> = BullDag::new();
//
//             let gvtx = Vertex::new(
//                 Block::Genesis {
//                     block: gblock.clone(),
//                 },
//                 gblock.hash.clone(),
//             );
//
//             let p1vtx = Vertex::new(
//                 Block::Proposal {
//                     block: prop1.clone(),
//                 },
//                 prop1.hash.clone(),
//             );
//
//             let p2vtx = Vertex::new(
//                 Block::Proposal {
//                     block: prop2.clone(),
//                 },
//                 prop2.hash.clone(),
//             );
//
//             let edges = vec![(&gvtx, &p1vtx), (&gvtx, &p2vtx)];
//
//             chain.extend_from_edges(edges);
//
//             let c_block =
//                 mine_convergence_block(&proposals, &chain, Block::Genesis {
// block: gblock });
//
//             if let Some(cb) = c_block {
//                 let sig = cb.header.miner_signature;
//                 let sig = Signature::from_str(&sig).unwrap();
//
//                 let payload = create_payload!(
//                     cb.header.ref_hashes,
//                     cb.header.round,
//                     cb.header.epoch,
//                     cb.header.block_seed,
//                     cb.header.next_block_seed,
//                     cb.header.block_height,
//                     cb.header.timestamp,
//                     cb.header.txn_hash,
//                     cb.header.miner_claim,
//                     cb.header.claim_list_hash,
//                     cb.header.block_reward,
//                     cb.header.next_block_reward
//                 );
//
//                 let mpk = cb.header.miner_claim.public_key;
//                 let mpk = PublicKey::from_str(&mpk).unwrap();
//
//                 let verify = sig.verify(&payload, &mpk);
//
//                 assert!(verify.is_ok())
//             }
//         } else {
//             panic!("A ConvergenceBlock should be produced")
//         }
//     }
//
//     #[test]
//     fn test_resolve_conflicts_curr_round() {
//         // Create a large transaction map
//         let genesis = mine_genesis();
//         if let Some(gblock) = genesis {
//             let ref_hash = gblock.hash.clone();
//             let round = gblock.header.round.clone() + 1;
//             let epoch = gblock.header.epoch.clone();
//
//             let mut prop1 = build_proposal_block(&ref_hash, 30, 10, round,
// epoch);
//
//             let mut prop2 = build_proposal_block(&ref_hash, 40, 5, round,
// epoch);
//
//             let txns: HashMap<String, Txn> = create_txns(5).collect();
//             prop1.txns.extend(txns.clone());
//             prop2.txns.extend(txns.clone());
//
//             let proposals = vec![prop1.clone(), prop2.clone()];
//
//             let mut chain: BullDag<Block, String> = BullDag::new();
//
//             let gvtx = Vertex::new(
//                 Block::Genesis {
//                     block: gblock.clone(),
//                 },
//                 gblock.hash.clone(),
//             );
//
//             let p1vtx = Vertex::new(
//                 Block::Proposal {
//                     block: prop1.clone(),
//                 },
//                 prop1.hash.clone(),
//             );
//
//             let p2vtx = Vertex::new(
//                 Block::Proposal {
//                     block: prop2.clone(),
//                 },
//                 prop2.hash.clone(),
//             );
//
//             let edges = vec![(&gvtx, &p1vtx), (&gvtx, &p2vtx)];
//
//             chain.extend_from_edges(edges);
//
//             let c_block =
//                 mine_convergence_block(&proposals, &chain, Block::Genesis {
// block: gblock });
//
//             if let Some(cb) = c_block {
//                 let sig = cb.header.miner_signature;
//                 let sig = Signature::from_str(&sig).unwrap();
//
//                 let payload = create_payload!(
//                     cb.header.ref_hashes,
//                     cb.header.round,
//                     cb.header.epoch,
//                     cb.header.block_seed,
//                     cb.header.next_block_seed,
//                     cb.header.block_height,
//                     cb.header.timestamp,
//                     cb.header.txn_hash,
//                     cb.header.miner_claim,
//                     cb.header.claim_list_hash,
//                     cb.header.block_reward,
//                     cb.header.next_block_reward
//                 );
//
//                 let mpk = cb.header.miner_claim.public_key;
//                 let mpk = PublicKey::from_str(&mpk).unwrap();
//
//                 let verify = sig.verify(&payload, &mpk);
//
//                 assert!(verify.is_ok());
//
//                 let total_w_duplicates = { prop1.txns.keys().len() +
// prop2.txns.keys().len() };
//
//                 assert!(total_w_duplicates > cb.txns.len());
//
//                 // Get the winner of the PoC election between proposer 1 and
// 2                 let mut proposer_ps = vec![
//                     (
//                         prop1.hash,
//                         prop1.from.get_pointer(cb.header.block_seed as u128),
//                     ),
//                     (
//                         prop2.hash,
//                         prop2.from.get_pointer(cb.header.block_seed as u128),
//                     ),
//                 ];
//
//                 // The first will be the winner
//                 proposer_ps.sort_unstable_by(|(_, a_pointer), (_, b_pointer)|
// {                     match (a_pointer, b_pointer) {
//                         (Some(x), Some(y)) => x.cmp(y),
//                         (None, Some(_)) => std::cmp::Ordering::Greater,
//                         (Some(_), None) => std::cmp::Ordering::Less,
//                         (None, None) => std::cmp::Ordering::Equal,
//                     }
//                 });
//                 let winner = proposer_ps[0].0.clone();
//                 let mut resolved_conflicts = cb.txns.clone();
//                 resolved_conflicts.retain(|_, set| {
//                     let conflicts = txns.keys().cloned().collect();
//                     let intersection: LinkedHashSet<&String> =
//                         set.intersection(&conflicts).collect();
//                     intersection.len() > 0
//                 });
//
//                 let key: Vec<String> =
// resolved_conflicts.keys().cloned().collect();
// assert!(key.len() == 1);                 assert_eq!(key[0], winner);
//             }
//         } else {
//             panic!("A ConvergenceBlock should be produced")
//         }
//     }
//
//     #[test]
//     fn test_resolve_conflicts_prev_rounds() {
//         let genesis = mine_genesis();
//         if let Some(gblock) = genesis {
//             let ref_hash = gblock.hash.clone();
//             let round = gblock.header.round.clone() + 1;
//             let epoch = gblock.header.epoch.clone();
//
//             let mut prop1 = build_proposal_block(&ref_hash, 30, 10, round,
// epoch);
//
//             let mut prop2 = build_proposal_block(&ref_hash, 40, 5, round,
// epoch);
//
//             let txns: HashMap<String, Txn> = create_txns(5).collect();
//             prop1.txns.extend(txns.clone());
//
//             let proposals = vec![prop1.clone(), prop2.clone()];
//
//             let mut chain: BullDag<Block, String> = BullDag::new();
//
//             let gvtx = Vertex::new(
//                 Block::Genesis {
//                     block: gblock.clone(),
//                 },
//                 gblock.hash.clone(),
//             );
//
//             let p1vtx = Vertex::new(
//                 Block::Proposal {
//                     block: prop1.clone(),
//                 },
//                 prop1.hash.clone(),
//             );
//
//             let p2vtx = Vertex::new(
//                 Block::Proposal {
//                     block: prop2.clone(),
//                 },
//                 prop2.hash.clone(),
//             );
//
//             let edges = vec![(&gvtx, &p1vtx), (&gvtx, &p2vtx)];
//
//             chain.extend_from_edges(edges);
//
//             let c_block_1 = mine_convergence_block(
//                 &proposals,
//                 &chain,
//                 Block::Genesis {
//                     block: gblock.clone(),
//                 },
//             );
//
//             let cb1 = c_block_1.unwrap();
//
//             let cb1vtx = Vertex::new(Block::Convergence { block: cb1.clone()
// }, cb1.hash.clone());
//
//             let edges = vec![(&p1vtx, &cb1vtx), (&p2vtx, &cb1vtx)];
//
//             chain.extend_from_edges(edges);
//
//             let mut prop3 = build_proposal_block(&ref_hash, 20, 10, round,
// epoch);
//
//             prop3.txns.extend(txns.clone());
//
//             let c_block_2 = mine_convergence_block(
//                 &vec![prop3.clone()],
//                 &chain,
//                 Block::Genesis {
//                     block: gblock.clone(),
//                 },
//             );
//
//             let cb2 = {
//                 let cb = c_block_2.unwrap();
//                 let sig = cb.header.miner_signature.clone();
//                 let sig = Signature::from_str(&sig).unwrap();
//
//                 let payload = create_payload!(
//                     cb.header.ref_hashes,
//                     cb.header.round,
//                     cb.header.epoch,
//                     cb.header.block_seed,
//                     cb.header.next_block_seed,
//                     cb.header.block_height,
//                     cb.header.timestamp,
//                     cb.header.txn_hash,
//                     cb.header.miner_claim,
//                     cb.header.claim_list_hash,
//                     cb.header.block_reward,
//                     cb.header.next_block_reward
//                 );
//
//                 let mpk = cb.header.miner_claim.public_key.clone();
//                 let mpk = PublicKey::from_str(&mpk).unwrap();
//
//                 let verify = sig.verify(&payload, &mpk);
//
//                 assert!(verify.is_ok());
//
//                 let total_w_duplicates = { prop3.txns.keys().len() };
//
//                 assert!(total_w_duplicates > cb.txns.len());
//
//                 cb
//             };
//
//             let p3vtx = Vertex::new(
//                 Block::Proposal {
//                     block: prop3.clone(),
//                 },
//                 prop3.hash.clone(),
//             );
//
//             let cb2vtx = Vertex::new(Block::Convergence { block: cb2.clone()
// }, cb2.hash.clone());
//
//             let edges = vec![(&gvtx, &p3vtx), (&p3vtx, &cb2vtx)];
//
//             chain.extend_from_edges(edges);
//         }
//     }
//
//     #[test]
//     fn test_epoch_change() {
//         let (msk1, mpk1) = create_keypair();
//         let addr = create_address(&mpk1);
//         let ref_hashes = vec!["abcdef".to_string()];
//         let epoch = 0;
//         let round = 29_999_998;
//         let block_seed = 34_989_333;
//         let next_block_seed = 839_999_843;
//         let block_height = 29_999_998;
//         let timestamp = utils::timestamp!();
//         let txn_hash = "abcdef01234567890".to_string();
//         let miner_claim = create_claim(&mpk1, &addr, &1);
//         let claim_list_hash = "01234567890abcdef".to_string();
//         let mut block_reward = Reward::default();
//         block_reward.current_block = block_height;
//         let next_block_reward = block_reward.clone();
//
//         let payload = create_payload!(
//             ref_hashes,
//             round,
//             epoch,
//             block_seed,
//             next_block_seed,
//             block_height,
//             timestamp,
//             txn_hash,
//             miner_claim,
//             claim_list_hash,
//             block_reward,
//             next_block_reward
//         );
//
//         let miner_signature = msk1.sign_ecdsa(payload).to_string();
//
//         let header = BlockHeader {
//             ref_hashes,
//             round,
//             epoch,
//             block_seed,
//             next_block_seed,
//             block_height,
//             timestamp,
//             txn_hash,
//             miner_claim,
//             claim_list_hash,
//             block_reward,
//             next_block_reward,
//             miner_signature,
//         };
//
//         let txns = LinkedHashMap::new();
//         let claims = LinkedHashMap::new();
//         let block_hash = hash_data!(
//             header.ref_hashes,
//             header.round,
//             header.epoch,
//             header.block_seed,
//             header.next_block_seed,
//             header.block_height,
//             header.timestamp,
//             header.txn_hash,
//             header.miner_claim,
//             header.claim_list_hash,
//             header.block_reward,
//             header.next_block_reward,
//             header.miner_signature
//         );
//
//         let cb1 = ConvergenceBlock {
//             header,
//             txns,
//             claims,
//             hash: block_hash,
//             certificate: None,
//         };
//
//         let mut chain: BullDag<Block, String> = BullDag::new();
//
//         let prop1 = build_proposal_block(&cb1.hash.clone(), 5, 5, 30_000_000,
// 0);         let cb1vtx = Vertex::new(Block::Convergence { block: cb1.clone()
// }, cb1.hash.clone());
//
//         let p1vtx = Vertex::new(
//             Block::Proposal {
//                 block: prop1.clone(),
//             },
//             prop1.hash.clone(),
//         );
//
//         let edges = vec![(&cb1vtx, &p1vtx)];
//
//         chain.extend_from_edges(edges);
//
//         let cb2 = mine_convergence_block_epoch_change(
//             &vec![prop1.clone()],
//             &chain,
//             &Block::Convergence { block: cb1.clone() },
//             0,
//         )
//         .unwrap();
//
//         assert_eq!(cb2.header.next_block_reward.epoch, 1);
//         assert_eq!(cb2.header.next_block_reward.next_epoch_block,
// 60_000_000);     }
//
//     #[test]
//     fn test_utility_adjustment() {
//         let (msk1, mpk1) = create_keypair();
//         let addr = create_address(&mpk1);
//         let ref_hashes = vec!["abcdef".to_string()];
//         let epoch = 0;
//         let round = 29_999_998;
//         let block_seed = 34_989_333;
//         let next_block_seed = 839_999_843;
//         let block_height = 29_999_998;
//         let timestamp = utils::timestamp!();
//         let txn_hash = "abcdef01234567890".to_string();
//         let miner_claim = create_claim(&mpk1, &addr, &1);
//         let claim_list_hash = "01234567890abcdef".to_string();
//         let mut block_reward = Reward::default();
//         block_reward.current_block = block_height;
//         let next_block_reward = block_reward.clone();
//
//         let payload = create_payload!(
//             ref_hashes,
//             round,
//             epoch,
//             block_seed,
//             next_block_seed,
//             block_height,
//             timestamp,
//             txn_hash,
//             miner_claim,
//             claim_list_hash,
//             block_reward,
//             next_block_reward
//         );
//
//         let miner_signature = msk1.sign_ecdsa(payload).to_string();
//
//         let header = BlockHeader {
//             ref_hashes,
//             round,
//             epoch,
//             block_seed,
//             next_block_seed,
//             block_height,
//             timestamp,
//             txn_hash,
//             miner_claim,
//             claim_list_hash,
//             block_reward,
//             next_block_reward,
//             miner_signature,
//         };
//
//         let txns = LinkedHashMap::new();
//         let claims = LinkedHashMap::new();
//         let block_hash = hash_data!(
//             header.ref_hashes,
//             header.round,
//             header.epoch,
//             header.block_seed,
//             header.next_block_seed,
//             header.block_height,
//             header.timestamp,
//             header.txn_hash,
//             header.miner_claim,
//             header.claim_list_hash,
//             header.block_reward,
//             header.next_block_reward,
//             header.miner_signature
//         );
//
//         let cb1 = ConvergenceBlock {
//             header,
//             txns,
//             claims,
//             hash: block_hash,
//             certificate: None,
//         };
//
//         let mut chain: BullDag<Block, String> = BullDag::new();
//
//         let prop1 = build_proposal_block(&cb1.hash.clone(), 5, 5, 30_000_000,
// 0);         let cb1vtx = Vertex::new(Block::Convergence { block: cb1.clone()
// }, cb1.hash.clone());
//
//         let p1vtx = Vertex::new(
//             Block::Proposal {
//                 block: prop1.clone(),
//             },
//             prop1.hash.clone(),
//         );
//
//         let edges = vec![(&cb1vtx, &p1vtx)];
//
//         chain.extend_from_edges(edges);
//
//         let cb2 = mine_convergence_block_epoch_change(
//             &vec![prop1.clone()],
//             &chain,
//             &Block::Convergence { block: cb1.clone() },
//             (4 * 30_000_000) as i128,
//         )
//         .unwrap();
//
//         assert_eq!(cb2.header.next_block_reward.amount, 24);
//     }
// }
