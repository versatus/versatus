pub mod miner;
pub mod result;
pub use crate::miner::*;
pub mod block_builder;
pub mod conflict_resolver;
pub mod miner_impl;
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
    #![allow(unused, deprecated, deprecated_in_future)]
    use bulldag::vertex::Vertex;
    use patriecia::common::Key;
    use primitives::Address;
    use ritelinked::LinkedHashMap;
    use secp256k1::Message;
    use vrrb_core::{keypair::Keypair, claim::Claim};
    use sha2::{Digest, Sha256};
    use block::{Block, ProposalBlock};
    use tokio;

    use crate::{
        test_helpers::{
            mine_genesis, 
            create_miner, 
            create_miner_from_keypair, 
            create_keypair, 
            create_and_sign_message, MinerDag, create_miner_return_dag, build_proposal_block, add_edges_to_dag
        }, MinerConfig
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

        let h_pk = proposal.from.public_key;
        let h_pk = PublicKey::from_str(&h_pk).unwrap();
        let sig = proposal.signature;
        let sig = Signature::from_str(&sig).unwrap();
        let verify = sig.verify(&payload, &h_pk);

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
    fn test_utility_adjustment() {
        let (msk1, mpk1) = create_keypair();

        assert_eq!(from_miner, sig);
            
        let valid = sig.verify(&msg, &kp.miner_kp.1);
        assert!(valid.is_ok());
    }

    #[test]
    fn test_mine_valid_convergence_block_empty_proposals() {
        let (mut miner, mut dag) = create_miner_return_dag();
        let keypair = Keypair::random();
        let other_miner = create_miner_from_keypair(&keypair); 
        
        let genesis = mine_genesis();
        if let Some(genesis) = genesis {
            miner.last_block = Some(&genesis);
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
        let genesis = mine_genesis();
    }

    #[test]
    fn test_mine_valid_convergence_block_from_proposals_conflicts_curr_round() {
        let genesis = mine_genesis();
    }

    #[test]
    fn test_mine_valid_convergence_block_from_proposals_conflicts_prev_rounds() {
        let genesis = mine_genesis();
    }

    #[test]
    fn test_miner_handles_epoch_change() {}

    #[test]
    fn test_miner_handles_utility_adjustment_upon_epoch_change() {}
}

pub(crate) mod test_helpers {
    use std::mem;

    use block::{
        invalid::InvalidBlockErrorReason,
        Block,
        ConvergenceBlock,
        GenesisBlock,
        ProposalBlock,
        TxnList, ConvergenceBlock,
    };
    use bulldag::{graph::BullDag, vertex::Vertex};
    use primitives::{Address, PublicKey, SecretKey, Signature};
    use secp256k1::Message;
    use sha256::digest;
    use utils::hash_data;
    use vrrb_core::{
        claim::Claim,
        helpers::size_of_txn_list,
        keypair::KeyPair,
        txn::{NewTxnArgs, Token, Txn},
    };

    use crate::{MineArgs, Miner, MinerConfig};

    /// Helper struct to build out DAG for testing 
    ///
    /// fields:
    ///     `genesis: Option<Vertex<Block, String>>`
    ///     `proposals: Vec<Vec<Vertex<Block, String>>>`
    ///     `convergence: Vec<Vec<Vertex<Block, String>>>`
    #[derive(Debug, Clone)]
    pub(crate) struct BatchEdges {
        genesis: Option<Vertex<Block, String>>,
        proposals: Vec<Vec<Vertex<Block, String>>>,
        convergence: Vec<Vertex<Block, String>>
    }

    pub(crate) fn create_miner() -> Miner<'static> {
        let (secret_key, public_key) = create_keypair();

        let address = create_address(&public_key).to_string();

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
        Address::new(pubkey.clone())
    }

    pub(crate) fn create_claim(pk: &PublicKey, addr: &str) -> Claim {
        Claim::new(pk.to_string(), addr.to_string())
    }

    pub(crate) fn mine_genesis() -> Option<GenesisBlock> {
        let (sk, pk) = create_keypair();
        let addr = create_address(&pk);
        let miner = create_miner();

        let claim = miner.generate_claim();

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
                let token = None;

                let txn_args = NewTxnArgs {
                    timestamp: 0,
                    sender_address: saddr.to_string(),
                    sender_public_key: pk.clone(),
                    receiver_address: raddr,
                    token,
                    amount,
                    signature: sk.sign_ecdsa(Message::from_hashed_data::<
                        secp256k1::hashes::sha256::Hash,
                    >(b"vrrb")),
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
                let claim = create_claim(&pk, &addr.to_string(), 1);
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
    ) -> Result<ProposalBlock, InvalidBlockErrorReason> {
        let (sk, pk) = create_keypair();
        let txns: TxnList = create_txns(n_tx).collect();

        let nonce = 1;

        let claims = create_claims(n_claims).collect();
        let hclaim = create_claim(&pk, &create_address(&pk).to_string(), 1);

        let miner = create_miner();

        let prop_block =
            miner.build_proposal_block(ref_hash.clone(), round, epoch, txns.clone(), claims, nonce);

        let total_txns_size = size_of_txn_list(&txns);

        if total_txns_size > 2000 {
            return Err(InvalidBlockErrorReason::InvalidBlockSize);
        }

        return prop_block;
    }

    pub(crate) fn mine_convergence_block(
        proposals: &Vec<ProposalBlock>,
        chain: &BullDag<Block, String>,
        last_block: Block,
    ) -> Option<ConvergenceBlock> {
        let (msk, mpk) = create_keypair();
        let maddr = create_address(&mpk).to_string();
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
        let maddr = create_address(&mpk).to_string();
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

    pub(crate) fn create_miner_return_dag() -> (Miner<'static>, MinerDag) {
        let mut miner = create_miner();
        let dag = miner.dag.clone();

        (miner, dag)
    }

    pub(crate) fn add_edges_to_dag(
        dag: &mut MinerDag, 
        genesis: bool, 
        n_proposals: u32, 
        n_rounds: u32, 
    ) {
        let mut batch = BatchEdges {
            genesis: None,
            proposals: vec![],
            convergence: vec![],
        };
        
        let genesis: Option<Vertex<Block, String>> = {
            if genesis {
                if let Some(genesis) = mine_genesis() {
                    let gblock = Block::Genesis { block: genesis.clone() };
                    Some(gblock.clone().into())
                } else {
                    None
                }
            } else {
                None 
            }
        };

        batch.genesis = genesis;

        if batch.genesis.is_none() {
            return 
        }

        for round in (0..n_rounds).into_iter() {  
            if round == 0 {
                match batch.genesis.clone().unwrap().get_data() {
                    Block::Genesis { ref block } => {
                        let proposals: Vec<Vertex<Block, String>> = (0..n_proposals)
                            .into_iter()
                            .map(|_| {
                                let proposal = build_proposal_block(
                                    &block.hash, 5, 4, round as u128, 0
                                    ).unwrap();  
                                let block = Block::from(proposal);
                                block.into()
                            }).collect();
    
                        batch.proposals.push(proposals);
                        
                        let mut miner = create_miner();
                        miner.dag = dag.clone();
                        if let Ok(block) = miner.try_mine() {
                            batch.convergence.push(block.into());
                        }
                    },
                    _ => {}
                }
            } else {
                match batch.convergence.clone()[round as usize].get_data() {
                    Block::Convergence { ref block } => {
                        let proposals: Vec<Vertex<Block, String>> = (0..n_proposals)
                            .into_iter()
                            .map(|_| {
                                let proposal = build_proposal_block(
                                    &block.hash, 5, 4, round as u128, 0
                                    ).unwrap();  
                                let block = Block::from(proposal);
                                block.into()
                            }).collect();
    
                        batch.proposals.push(proposals);

                        let mut miner = create_miner();
                        miner.dag = dag.clone();
                        if let Ok(block) = miner.try_mine() {
                            batch.convergence.push(block.into());
                        }
                    },
                    _ => {}
                }
            }
        }
    }
}
