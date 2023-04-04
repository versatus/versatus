pub mod miner;
pub mod result;
pub use crate::miner::*;
pub mod block_builder;
pub mod conflict_resolver;
pub mod miner_impl;
pub(crate) mod test_helpers;
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
    use std::sync::Arc;

    use bulldag::vertex::Vertex;
    use primitives::Address;
    use ritelinked::LinkedHashMap;
    use vrrb_core::{keypair::Keypair, claim::Claim, txn::{TransactionDigest, Txn}};
    use block::{Block, ProposalBlock};

    use crate::test_helpers::{
        mine_genesis, 
        create_miner, 
        create_miner_from_keypair, 
        create_and_sign_message, 
        create_miner_return_dag, build_single_proposal_block, 
        create_miner_from_keypair_and_dag, 
        create_miner_from_keypair_return_dag, 
        build_single_proposal_block_from_txns, 
        create_txns,
    };

    #[test]
    fn test_create_genesis_block() {
        let genesis = mine_genesis();
        assert!(genesis.is_some());
    }

    #[test]
    fn test_create_proposal_block() {
        let genesis = mine_genesis().unwrap();
        let proposal = build_proposal_block(&genesis.hash, 30, 10, 0, 0).unwrap();

        let payload = create_payload!(
            proposal.round,
            proposal.epoch,
            proposal.txns,
            proposal.claims,
            proposal.from
        );

    #[test]
    fn test_get_miner_publickey() {
        let kp = Keypair::random();
        let miner = create_miner_from_keypair(&kp);

        assert!(verify.is_ok())
    }

    #[test]
    fn test_create_proposal_block_over_size_limit() {
        let genesis = mine_genesis().unwrap();
        let proposal = build_proposal_block(&genesis.hash, 2000, 10, 0, 0);

        assert!(proposal.is_err());
    }

    #[test]
    fn test_create_convergence_block_no_conflicts() {
        let genesis = mine_genesis();
        if let Some(gblock) = genesis {
            let ref_hash = gblock.hash.clone();
            let round = gblock.header.round.clone() + 1;
            let epoch = gblock.header.epoch.clone();

            let prop1 = build_proposal_block(&ref_hash, 30, 10, round, epoch)
                .unwrap()
                .clone();

            let prop2 = build_proposal_block(&ref_hash, 40, 5, round, epoch)
                .unwrap()
                .clone();

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

            let mut prop1 = build_proposal_block(&ref_hash, 30, 10, round, epoch)
                .unwrap()
                .clone();

            let mut prop2 = build_proposal_block(&ref_hash, 40, 5, round, epoch)
                .unwrap()
                .clone();

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

            let mut prop1 = build_proposal_block(&ref_hash, 30, 10, round, epoch)
                .unwrap()
                .clone();

            let mut prop2 = build_proposal_block(&ref_hash, 40, 5, round, epoch)
                .unwrap()
                .clone();

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

            let mut prop3 = build_proposal_block(&ref_hash, 20, 10, round, epoch)
                .unwrap()
                .clone();

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
        let timestamp = chrono::Utc::now().timestamp();
        let txn_hash = "abcdef01234567890".to_string();
        let miner_claim = miner.generate_claim();
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

        let prop1 = build_proposal_block(&cb1.hash.clone(), 5, 5, 30_000_000, 0)
            .unwrap()
            .clone();

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
    fn test_sign_valid_message() {
        let (msg, kp, sig) = create_and_sign_message();
        let miner = create_miner_from_keypair(&kp);
        let from_miner = miner.sign_message(msg.clone());

        assert_eq!(from_miner, sig);
            
        let valid = sig.verify(&msg, &kp.miner_kp.1);
        assert!(valid.is_ok());
    }

    #[test]
    fn test_mine_valid_convergence_block_empty_proposals() {
        let (mut miner, dag) = create_miner_return_dag();
        let keypair = Keypair::random();
        let other_miner = create_miner_from_keypair(&keypair); 
        
        let genesis = mine_genesis();
        if let Some(genesis) = genesis {
            miner.last_block = Some(Arc::new(genesis.clone()));
            let gblock = Block::Genesis { block: genesis.clone() };
            let gvtx: Vertex<Block, String> = gblock.into();
            let prop1 = ProposalBlock::build(
                genesis.hash.clone(),
                0,
                0,
                LinkedHashMap::new(),
                LinkedHashMap::new(),
                other_miner.claim.clone(),
                keypair.miner_kp.0.clone()
            );
            let pblock = Block::Proposal { block: prop1.clone() };
            let pvtx: Vertex<Block, String> = pblock.into(); 
            if let Ok(mut guard) = dag.write() {
                let edge = (&gvtx, &pvtx);
                guard.add_edge(edge);
            }

            let convergence = miner.try_mine(); 
            if let Ok(cblock) = convergence {
                let cvtx: Vertex<Block, String> = cblock.into();
                if let Ok(mut guard) = dag.write() {
                    let edge = (&pvtx, &cvtx);
                    guard.add_edge(edge);
                }
            }

            if let Ok(guard) = dag.read() {
                assert_eq!(guard.len(), 3);
            }
        }
    }

    #[test]
    fn test_mine_valid_convergence_block_from_proposals_w_no_conflicts() {
        let m1kp = Keypair::random();
        let m2kp = Keypair::random();
        let (mut miner, dag) = create_miner_from_keypair_return_dag(&m1kp); 
        let mut other_miner = create_miner_from_keypair_and_dag(
            &m2kp, 
            dag.clone()
        ); 
        
        let genesis = mine_genesis();
        if let Some(genesis) = genesis {
            miner.last_block = Some(Arc::new(genesis.clone()));
            other_miner.last_block = Some(Arc::new(genesis.clone()));
            let gblock = Block::Genesis { block: genesis.clone() };
            let gvtx: Vertex<Block, String> = gblock.into();
            let prop1 = build_single_proposal_block(
                genesis.hash.clone(),
                5,
                4,
                0,
                0,
                miner.claim.clone(),
                m1kp.miner_kp.0.clone()
            );
            let prop2 = build_single_proposal_block(
                genesis.hash.clone(),
                5,
                4,
                0,
                0,
                other_miner.claim.clone(),
                m2kp.miner_kp.0.clone()
            );

            let pblock1 = Block::Proposal { block: prop1.clone() };
            let pvtx1: Vertex<Block, String> = pblock1.into(); 
            let pblock2 = Block::Proposal { block: prop2.clone() };
            let pvtx2: Vertex<Block, String> = pblock2.into(); 
            if let Ok(mut guard) = dag.write() {
                let edge1 = (&gvtx, &pvtx1);
                let edge2 = (&gvtx, &pvtx2);
                guard.add_edge(edge1);
                guard.add_edge(edge2);
            }

            let convergence = miner.try_mine(); 
            if let Ok(cblock) = convergence {
                let cvtx: Vertex<Block, String> = cblock.into();
                if let Ok(mut guard) = dag.write() {
                    let edge1 = (&pvtx1, &cvtx);
                    let edge2 = (&pvtx2, &cvtx);
                    guard.add_edge(edge1);
                    guard.add_edge(edge2);
                }
            }

            if let Ok(guard) = dag.read() {
                assert_eq!(guard.len(), 4);
            }
        }
    }

    #[test]
    fn test_mine_valid_convergence_block_from_proposals_conflicts_curr_round() {
        let m1kp = Keypair::random();
        let (mut miner, dag) = create_miner_from_keypair_return_dag(&m1kp); 
        
        let genesis = mine_genesis();
        if let Some(genesis) = genesis {
            miner.last_block = Some(Arc::new(genesis.clone()));
            let gblock = Block::Genesis { block: genesis.clone() };
            let gvtx: Vertex<Block, String> = gblock.into();
            let txns: LinkedHashMap<TransactionDigest, Txn> = create_txns(5).collect();
            let prop1 = build_single_proposal_block_from_txns(
                genesis.hash.clone(), txns.clone(), 0, 0
            );
            let prop2 = build_single_proposal_block_from_txns(
                genesis.hash.clone(), txns.clone(), 0, 0
            );

            let pblock1 = Block::Proposal { block: prop1.clone() };
            let pvtx1: Vertex<Block, String> = pblock1.into(); 
            let pblock2 = Block::Proposal { block: prop2.clone() };
            let pvtx2: Vertex<Block, String> = pblock2.into(); 
            if let Ok(mut guard) = dag.write() {
                let edge1 = (&gvtx, &pvtx1);
                let edge2 = (&gvtx, &pvtx2);
                guard.add_edge(edge1);
                guard.add_edge(edge2);
            }

            let convergence = miner.try_mine(); 
            if let Ok(cblock) = convergence {
                if let Block::Convergence { block } = cblock.clone() {
                    miner.last_block = Some(Arc::new(block));
                }
                let cvtx: Vertex<Block, String> = cblock.clone().into();
                if let Ok(mut guard) = dag.write() {
                    let edge1 = (&pvtx1, &cvtx);
                    let edge2 = (&pvtx2, &cvtx);
                    guard.add_edge(edge1);
                    guard.add_edge(edge2);
                }

                match cblock {
                    Block::Convergence { ref block } => {
                        let total_len: usize = block.txns.iter().map(|(_, v)| {v.len()}).sum();
                        assert_eq!(total_len, 15usize);
                    },
                    _ => {}
                }
            }

            if let Ok(guard) = dag.read() {
                assert_eq!(guard.len(), 4);
            }
        }
    }

    #[test]
    fn test_mine_valid_convergence_block_from_proposals_conflicts_prev_rounds() {
        let m1kp = Keypair::random();
        let (mut miner, dag) = create_miner_from_keypair_return_dag(&m1kp); 
        
        let genesis = mine_genesis();
        if let Some(genesis) = genesis {
            miner.last_block = Some(Arc::new(genesis.clone()));
            let gblock = Block::Genesis { block: genesis.clone() };
            let gvtx: Vertex<Block, String> = gblock.into();
            let txns: LinkedHashMap<TransactionDigest, Txn> = create_txns(5).collect();
            let prop1 = build_single_proposal_block_from_txns(
                genesis.hash.clone(), txns.clone(), 0, 0
            );
            let pblock1 = Block::Proposal { block: prop1.clone() };
            let pvtx1: Vertex<Block, String> = pblock1.into(); 
            if let Ok(mut guard) = dag.write() {
                let edge1 = (&gvtx, &pvtx1);
                guard.add_edge(edge1);
            }

            let convergence = miner.try_mine(); 
            if let Ok(Block::Convergence { ref block }) = convergence {
                miner.last_block = Some(Arc::new(block.to_owned()));
                let cvtx1: Vertex<Block, String> = Block::Convergence { block: block.clone() }.into();
                if let Ok(mut guard) = dag.write() {
                    let edge1 = (&pvtx1, &cvtx1);
                    guard.add_edge(edge1);
                }
            };


            let prop2 = build_single_proposal_block_from_txns(
                genesis.hash.clone(), txns.clone(), 0, 0
            );
            let pblock2 = Block::Proposal { block: prop2.clone() };
            let pvtx2: Vertex<Block, String> = pblock2.into(); 

            if let Ok(mut guard) = dag.write() {
                let edge2 = (&gvtx, &pvtx2);
                guard.add_edge(edge2);
            }

            let convergence = miner.try_mine(); 
            if let Ok(Block::Convergence { ref block }) = convergence {
                miner.last_block = Some(Arc::new(block.to_owned()));
                let cvtx2: Vertex<Block, String> = Block::Convergence { block: block.clone() }.into();
                if let Ok(mut guard) = dag.write() {
                    let edge2 = (&pvtx2, &cvtx2);
                    guard.add_edge(edge2);
                }

                match convergence {
                    Ok(Block::Convergence { ref block }) => {
                        let total_len: usize = block.txns.iter().map(|(_, v)| {v.len()}).sum();
                        assert_eq!(total_len, 5usize);
                    },
                    _ => {}
                }
            }

            if let Ok(guard) = dag.read() {
                assert_eq!(guard.len(), 5);
            }
        }
    }

    #[test]
    fn test_miner_handles_epoch_change() {
        let m1kp = Keypair::random();
        let (mut miner, dag) = create_miner_from_keypair_return_dag(&m1kp); 
        
        let genesis = mine_genesis();
        if let Some(genesis) = genesis {
            miner.last_block = Some(Arc::new(genesis.clone()));
            let gblock = Block::Genesis { block: genesis.clone() };
            let gvtx: Vertex<Block, String> = gblock.into();
            let txns: LinkedHashMap<TransactionDigest, Txn> = create_txns(5).collect();
            let prop1 = build_single_proposal_block_from_txns(
                genesis.hash.clone(), txns.clone(), 0, 0
            );
            let pblock1 = Block::Proposal { block: prop1.clone() };
            let pvtx1: Vertex<Block, String> = pblock1.into(); 
            if let Ok(mut guard) = dag.write() {
                let edge1 = (&gvtx, &pvtx1);
                guard.add_edge(edge1);
            }

            let convergence = miner.try_mine(); 
            if let Ok(Block::Convergence { mut block }) = convergence {
                block.header.round = 29_999_998;
                block.header.block_height = 29_999_998;
                block.header.block_reward.current_block = 29_999_998;
                miner.last_block = Some(Arc::new(block.to_owned()));
                let cvtx1: Vertex<Block, String> = Block::Convergence { block: block.clone() }.into();
                if let Ok(mut guard) = dag.write() {
                    let edge1 = (&pvtx1, &cvtx1);
                    guard.add_edge(edge1);
                }
            };


            let convergence = miner.try_mine(); 
            if let Ok(Block::Convergence { ref block }) = convergence {
                miner.last_block = Some(Arc::new(block.to_owned()));
                assert_eq!(1, block.header.next_block_reward.epoch);
            }
        }
    }

    #[test]
    fn test_miner_handles_utility_adjustment_upon_epoch_change() {

        let m1kp = Keypair::random();
        let (mut miner, dag) = create_miner_from_keypair_return_dag(&m1kp); 
        
        let genesis = mine_genesis();
        if let Some(genesis) = genesis {
            miner.last_block = Some(Arc::new(genesis.clone()));
            let gblock = Block::Genesis { block: genesis.clone() };
            let gvtx: Vertex<Block, String> = gblock.into();
            let txns: LinkedHashMap<TransactionDigest, Txn> = create_txns(5).collect();
            let prop1 = build_single_proposal_block_from_txns(
                genesis.hash.clone(), txns.clone(), 0, 0
            );
            let pblock1 = Block::Proposal { block: prop1.clone() };
            let pvtx1: Vertex<Block, String> = pblock1.into(); 
            if let Ok(mut guard) = dag.write() {
                let edge1 = (&gvtx, &pvtx1);
                guard.add_edge(edge1);
            }

            miner.set_next_epoch_adjustment(30_000_000_i128);

            let convergence = miner.try_mine(); 
            if let Ok(Block::Convergence { mut block }) = convergence {
                block.header.round = 29_999_998;
                block.header.block_height = 29_999_998;
                block.header.block_reward.current_block = 29_999_998;
                miner.last_block = Some(Arc::new(block.to_owned()));
                let cvtx1: Vertex<Block, String> = Block::Convergence { block: block.clone() }.into();
                if let Ok(mut guard) = dag.write() {
                    let edge1 = (&pvtx1, &cvtx1);
                    guard.add_edge(edge1);
                }
            };


            let convergence = miner.try_mine(); 
            if let Ok(Block::Convergence { ref block }) = convergence {
                miner.last_block = Some(Arc::new(block.to_owned()));
                assert_eq!(1, block.header.next_block_reward.epoch);
                assert_eq!(21, block.header.next_block_reward.amount);
            }
        }
    }
}

