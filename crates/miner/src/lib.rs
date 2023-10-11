pub mod miner;
pub mod result;
pub use crate::miner::*;
pub mod block_builder;
pub mod conflict_resolver;
pub mod miner_impl;
pub mod test_helpers;

pub mod v2 {
    pub use crate::miner::*;
}

#[cfg(test)]
mod tests {
    use std::{net::SocketAddr, sync::Arc};

    use block::{Block, ProposalBlock};
    use bulldag::vertex::Vertex;
    use primitives::Address;
    use ritelinked::LinkedHashMap;
    use vrrb_core::transactions::{TransactionDigest, TransactionKind};
    use vrrb_core::{claim::Claim, keypair::Keypair};

    use crate::test_helpers::{
        build_single_proposal_block, build_single_proposal_block_from_txns,
        create_and_sign_message, create_miner, create_miner_from_keypair,
        create_miner_from_keypair_and_dag, create_miner_from_keypair_return_dag,
        create_miner_return_dag, create_txns, mine_genesis,
    };

    #[test]
    fn test_create_miner() {
        let kp = Keypair::random();
        let address = Address::new(kp.miner_kp.1.clone());
        let ip_address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
        let signature = Claim::signature_for_valid_claim(
            kp.miner_kp.1.clone(),
            ip_address.clone(),
            kp.get_miner_secret_key().secret_bytes().to_vec(),
        )
        .unwrap();
        let claim = Claim::new(
            kp.miner_kp.1.clone(),
            address.clone(),
            ip_address,
            signature,
            "test-miner-node".into(),
        )
        .unwrap();
        let miner = create_miner_from_keypair(&kp);

        assert_eq!(miner.claim, claim);
    }

    #[test]
    fn test_get_miner_address() {
        let kp = Keypair::random();
        let address = Address::new(kp.miner_kp.1.clone());
        let miner = create_miner_from_keypair(&kp);

        assert_eq!(miner.address(), address);
    }

    #[test]
    fn test_get_miner_publickey() {
        let kp = Keypair::random();
        let miner = create_miner_from_keypair(&kp);

        assert_eq!(miner.public_key(), kp.miner_kp.1);
    }

    #[test]
    fn test_read_miner_dag_copy() {
        let miner = create_miner();
        let read_guard = miner.dag.read();

        assert!(read_guard.is_ok());
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
        let engine = signer::engine::SignerEngine::new(
            keypair.get_miner_public_key().clone(),
            keypair.get_miner_secret_key().clone(),
        );

        let genesis = mine_genesis();
        if let Some(genesis) = genesis {
            miner.last_block = Some(Arc::new(genesis.clone()));
            let gblock = Block::Genesis {
                block: genesis.clone(),
            };
            let gvtx: Vertex<Block, String> = gblock.into();
            let prop1 = ProposalBlock::build(
                genesis.hash.clone(),
                0,
                0,
                LinkedHashMap::new(),
                LinkedHashMap::new(),
                other_miner.claim.clone(),
                engine.clone(),
            );
            let pblock = Block::Proposal {
                block: prop1.clone(),
            };
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
        let mut other_miner = create_miner_from_keypair_and_dag(&m2kp, dag.clone());
        let engine1 = signer::engine::SignerEngine::new(
            m1kp.get_miner_public_key().clone(),
            m1kp.get_miner_secret_key().clone(),
        );
        let engine2 = signer::engine::SignerEngine::new(
            m2kp.get_miner_public_key().clone(),
            m2kp.get_miner_secret_key().clone(),
        );

        let genesis = mine_genesis();
        if let Some(genesis) = genesis {
            miner.last_block = Some(Arc::new(genesis.clone()));
            other_miner.last_block = Some(Arc::new(genesis.clone()));
            let gblock = Block::Genesis {
                block: genesis.clone(),
            };
            let gvtx: Vertex<Block, String> = gblock.into();
            let prop1 = build_single_proposal_block(
                genesis.hash.clone(),
                5,
                4,
                0,
                0,
                miner.claim.clone(),
                engine1,
            );
            let prop2 = build_single_proposal_block(
                genesis.hash.clone(),
                5,
                4,
                0,
                0,
                other_miner.claim.clone(),
                engine2,
            );

            let pblock1 = Block::Proposal {
                block: prop1.clone(),
            };
            let pvtx1: Vertex<Block, String> = pblock1.into();
            let pblock2 = Block::Proposal {
                block: prop2.clone(),
            };
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
            let gblock = Block::Genesis {
                block: genesis.clone(),
            };
            let gvtx: Vertex<Block, String> = gblock.into();
            let txns: LinkedHashMap<TransactionDigest, TransactionKind> = create_txns(5).collect();
            let prop1 =
                build_single_proposal_block_from_txns(genesis.hash.clone(), txns.clone(), 0, 0);
            let prop2 =
                build_single_proposal_block_from_txns(genesis.hash.clone(), txns.clone(), 0, 0);

            let pblock1 = Block::Proposal {
                block: prop1.clone(),
            };
            let pvtx1: Vertex<Block, String> = pblock1.into();
            let pblock2 = Block::Proposal {
                block: prop2.clone(),
            };
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
                        let total_len: usize = block.txns.iter().map(|(_, v)| v.len()).sum();
                        assert_eq!(total_len, 15usize);
                    },
                    _ => {},
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
            let gblock = Block::Genesis {
                block: genesis.clone(),
            };
            let gvtx: Vertex<Block, String> = gblock.into();
            let txns: LinkedHashMap<TransactionDigest, TransactionKind> = create_txns(5).collect();
            let prop1 =
                build_single_proposal_block_from_txns(genesis.hash.clone(), txns.clone(), 0, 0);
            let pblock1 = Block::Proposal {
                block: prop1.clone(),
            };
            let pvtx1: Vertex<Block, String> = pblock1.into();
            if let Ok(mut guard) = dag.write() {
                let edge1 = (&gvtx, &pvtx1);
                guard.add_edge(edge1);
            }

            let convergence = miner.try_mine();
            if let Ok(Block::Convergence { ref block }) = convergence {
                miner.last_block = Some(Arc::new(block.to_owned()));
                let cvtx1: Vertex<Block, String> = Block::Convergence {
                    block: block.clone(),
                }
                .into();
                if let Ok(mut guard) = dag.write() {
                    let edge1 = (&pvtx1, &cvtx1);
                    guard.add_edge(edge1);
                }
            };

            let prop2 =
                build_single_proposal_block_from_txns(genesis.hash.clone(), txns.clone(), 0, 0);
            let pblock2 = Block::Proposal {
                block: prop2.clone(),
            };
            let pvtx2: Vertex<Block, String> = pblock2.into();

            if let Ok(mut guard) = dag.write() {
                let edge2 = (&gvtx, &pvtx2);
                guard.add_edge(edge2);
            }

            let convergence = miner.try_mine();
            if let Ok(Block::Convergence { ref block }) = convergence {
                miner.last_block = Some(Arc::new(block.to_owned()));
                let cvtx2: Vertex<Block, String> = Block::Convergence {
                    block: block.clone(),
                }
                .into();
                if let Ok(mut guard) = dag.write() {
                    let edge2 = (&pvtx2, &cvtx2);
                    guard.add_edge(edge2);
                }

                match convergence {
                    Ok(Block::Convergence { ref block }) => {
                        let total_len: usize = block.txns.iter().map(|(_, v)| v.len()).sum();
                        assert_eq!(total_len, 5usize);
                    },
                    _ => {},
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
            let gblock = Block::Genesis {
                block: genesis.clone(),
            };
            let gvtx: Vertex<Block, String> = gblock.into();
            let txns: LinkedHashMap<TransactionDigest, TransactionKind> = create_txns(5).collect();
            let prop1 =
                build_single_proposal_block_from_txns(genesis.hash.clone(), txns.clone(), 0, 0);
            let pblock1 = Block::Proposal {
                block: prop1.clone(),
            };
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
                let cvtx1: Vertex<Block, String> = Block::Convergence {
                    block: block.clone(),
                }
                .into();
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
            let gblock = Block::Genesis {
                block: genesis.clone(),
            };
            let gvtx: Vertex<Block, String> = gblock.into();
            let txns: LinkedHashMap<TransactionDigest, TransactionKind> = create_txns(5).collect();
            let prop1 =
                build_single_proposal_block_from_txns(genesis.hash.clone(), txns.clone(), 0, 0);
            let pblock1 = Block::Proposal {
                block: prop1.clone(),
            };
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
                let cvtx1: Vertex<Block, String> = Block::Convergence {
                    block: block.clone(),
                }
                .into();
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
