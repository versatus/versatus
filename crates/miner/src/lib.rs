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
            create_and_sign_message, MinerDag, create_miner_return_dag, build_proposal_block, create_txns
        }, MinerConfig
    };

    #[test]
    fn test_create_miner() {
        let kp = Keypair::random();
        let address = Address::new(kp.miner_kp.1.clone());
        let claim = Claim::new(kp.miner_kp.1.to_string().clone(), address.to_string().clone());
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
        let address = Address::new(kp.miner_kp.1.clone());
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
        let mut miner = create_miner_from_keypair(&kp);
        let from_miner = miner.sign_message(msg.clone());

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
            let pblock1 = Block::Proposal { block: prop1.clone() };
            let pvtx1: Vertex<Block, String> = pblock1.into(); 
            if let Ok(mut guard) = dag.write() {
                let edge = (&gvtx, &pvtx1);
                guard.add_edge(edge);
            }

            let convergence = miner.try_mine(); 
            if let Ok(cblock) = convergence {
                let cvtx: Vertex<Block, String> = cblock.into();
                if let Ok(mut guard) = dag.write() {
                    let edge = (&pvtx1, &cvtx);
                    guard.add_edge(edge);
                }
            }

            if let Ok(guard) = dag.read() {
                assert_eq!(guard.len(), 3);
            }
        }
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
    #![allow(unused, deprecated, deprecated_in_future)]
    use std::sync::{Arc, RwLock};

    use block::{
        invalid::InvalidBlockErrorReason,
        Block,
        GenesisBlock,
        ProposalBlock,
        TxnList, ConvergenceBlock, InnerBlock,
    };
    use bulldag::{graph::BullDag, vertex::Vertex};
    use primitives::{Address, PublicKey, SecretKey, Signature};
    use ritelinked::LinkedHashMap;
    use secp256k1::Message;
    use sha2::{Digest, Sha256};
    use vrrb_core::{
        claim::Claim,
        helpers::size_of_txn_list,
        keypair::Keypair,
        txn::{NewTxnArgs, Txn, TransactionDigest, generate_txn_digest_vec},
    };

    use crate::{Miner, MinerConfig, result::MinerError};

    /// Move this into primitives and call it simply `BlockDag`
    pub type MinerDag = Arc<RwLock<BullDag<Block, String>>>;

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
        let dag: MinerDag = Arc::new(RwLock::new(BullDag::new()));

        let config = MinerConfig {
            secret_key,
            public_key,
            dag,
        };

        Miner::new(config)
    }

    pub(crate) fn create_miner_from_keypair(kp: &Keypair) -> Miner {
        let (secret_key, public_key) = kp.miner_kp;
        let dag: MinerDag = Arc::new(RwLock::new(BullDag::new()));
        
        let config = MinerConfig {
            secret_key,
            public_key,
            dag
        };

        Miner::new(config)
    }

    pub(crate) fn create_keypair() -> (SecretKey, PublicKey) {
        let kp = Keypair::random();
        kp.miner_kp
    }

    pub(crate) fn create_address(pubkey: &PublicKey) -> Address {
        Address::new(pubkey.clone())
    }

    pub(crate) fn create_claim(pk: &PublicKey, addr: &str) -> Claim {
        Claim::new(pk.to_string(), addr.to_string())
    }

    pub(crate) fn create_and_sign_message() -> (Message, Keypair, Signature) {
        let kp = Keypair::random();
        let message = b"Test Message";
        let msg = {
            let mut hasher = sha2::Sha256::new();
            hasher.update(message);
            let message = hasher.finalize();
            Message::from_slice(&message[..]).unwrap()
        };

        let sig = kp.miner_kp.0.sign_ecdsa(msg);

        return (msg, kp, sig)

    }

    pub(crate) fn mine_genesis() -> Option<GenesisBlock> {
        let miner = create_miner();

        let claim = miner.generate_claim();

        let claim_list = {
            vec![(claim.public_key.clone(), claim.clone())]
                .iter()
                .cloned()
                .collect()
        };

        miner.mine_genesis_block(claim_list)
    }

    pub(crate) fn create_txns(n: usize) -> impl Iterator<Item = (TransactionDigest, Txn)> {
        (0..n)
            .map(|n| {
                let (sk, pk) = create_keypair();
                let raddr = "0x192abcdef01234567890".to_string();
                let saddr = create_address(&pk);
                let amount = (n.pow(2)) as u128;
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

                let txn_digest_vec = generate_txn_digest_vec(
                    txn.timestamp, 
                    txn.sender_address.clone(), 
                    txn.sender_public_key.clone(), 
                    txn.receiver_address.clone(), 
                    txn.token.clone(), 
                    txn.amount, 
                    txn.nonce
                ); 

                let digest = TransactionDigest::from(txn_digest_vec);

                (digest, txn)
            })
            .into_iter()
    }

    pub(crate) fn create_claims(n: usize) -> impl Iterator<Item = (String, Claim)> {
        (0..n)
            .map(|_| {
                let (_, pk) = create_keypair();
                let addr = create_address(&pk);
                let claim = create_claim(&pk, &addr.to_string());
                (claim.public_key.clone(), claim)
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
        let txns: TxnList = create_txns(n_tx).collect();

        let claims = create_claims(n_claims).collect();

        let miner = create_miner();

        let prop_block =
            miner.build_proposal_block(ref_hash.clone(), round, epoch, txns.clone(), claims);

        let total_txns_size = size_of_txn_list(&txns);

        if total_txns_size > 2000 {
            return Err(InvalidBlockErrorReason::InvalidBlockSize);
        }

        return prop_block;
    }

    pub(crate) fn mine_convergence_block() -> Result<Block, MinerError> {
        let mut miner = create_miner();
        miner.try_mine()
    }

    pub(crate) fn mine_convergence_block_epoch_change(
    ) -> Result<Block, MinerError> {
        let mut miner = create_miner();
        //TODO: Add Mock Convergence Block with round height of 29.999999mm
        miner.try_mine()
    }

    pub(crate) fn create_miner_return_dag() -> (Miner<'static>, MinerDag) {
        let mut miner = create_miner();
        let dag = miner.dag.clone();

        (miner, dag)
    }

    pub(crate) fn create_miner_from_dag(dag: MinerDag) -> (Miner<'static>, MinerDag) {
        let mut miner = create_miner();
        miner.dag = dag.clone(); 

        (miner, dag)
    }

    pub(crate) fn build_single_proposal_block(
        last_block_hash: String,
        n_txns: usize, 
        n_claims: usize,
        round: u128,
        epoch: u128,
        from: Claim,
        sk: SecretKey,
    ) -> ProposalBlock {
        let txns = create_txns(n_txns).collect();
        let claims = create_claims(n_claims).collect();
        ProposalBlock::build(
            last_block_hash,
            round,
            epoch,
            txns,
            claims,
            from,
            sk 
        )
    }

    pub(crate) fn build_multiple_proposal_blocks_single_round(
        n_blocks: usize,
        last_block_hash: String,
        n_txns: usize,
        n_claims: usize,
        round: u128,
        epoch: u128,
    ) -> Vec<ProposalBlock> {
        
        (0..n_blocks).into_iter().map(|n| {

            let keypair = Keypair::random();
            let address = Address::new(keypair.miner_kp.1.clone());
            let claim = Claim::new(keypair.miner_kp.1.clone().to_string(), address.to_string());
            let prop = build_single_proposal_block(
                last_block_hash.clone(), 
                n_txns, 
                n_claims, 
                round, 
                epoch, 
                claim, 
                keypair.miner_kp.0.clone()
            );
            prop
        }).collect()
    }

    pub(crate) fn build_multiple_rounds(
        dag: &mut MinerDag,
        last_block_hash: String, 
        n_blocks: usize, 
        n_txns: usize,
        n_claims: usize,
        n_rounds: usize,
        round: &mut usize,
        epoch: usize,
    ) {
        // Recursively add rounds of blocks to the DAG
        // Need to ensure that there is a genesis block in the dag already.
        if n_rounds > round.clone() {
            if dag_has_genesis(&mut dag.clone()) {
                if let Some(hash) = mine_next_convergence_block(&mut dag.clone()) {
                    *round += 1usize;
                    let proposals = build_multiple_proposal_blocks_single_round(
                        n_blocks, hash.clone(), n_txns, n_claims, round.clone() as u128, epoch as u128
                    );
                    append_proposal_blocks_to_dag(&mut dag.clone(), proposals);
                    build_multiple_rounds(
                        &mut dag.clone(),
                        hash,
                        n_blocks,
                        n_txns,
                        n_claims,
                        n_rounds,
                        round,
                        epoch,
                    );
                };
                
            } else {
                if let Some(hash) = add_genesis_to_dag(&mut dag.clone()) {
                    *round += 1usize;
                    build_multiple_rounds(
                        &mut dag.clone(),
                        hash,
                        n_blocks,
                        n_txns,
                        n_claims,
                        n_rounds,
                        round,
                        epoch
                    );
                }
            }
        }
    }

    pub(crate) fn dag_has_genesis(dag: &mut MinerDag) -> bool {
        dag.read().unwrap().len() > 0
    }

    pub(crate) fn add_genesis_to_dag(dag: &mut MinerDag) -> Option<String> {
        let mut prop_vertices = Vec::new();
        let genesis = mine_genesis();
        let keypair = Keypair::random();
        let miner = create_miner_from_keypair(&keypair);

        if let Some(genesis) = genesis {
            let gblock = Block::Genesis { block: genesis.clone() };
            let gvtx: Vertex<Block, String> = gblock.into();
            let prop1 = ProposalBlock::build(
                genesis.hash.clone(),
                0,
                0,
                LinkedHashMap::new(),
                LinkedHashMap::new(),
                miner.claim.clone(),
                keypair.miner_kp.0.clone()
            );
            let pblock = Block::Proposal { block: prop1.clone() };
            let pvtx: Vertex<Block, String> = pblock.into(); 
            prop_vertices.push(pvtx.clone());
            if let Ok(mut guard) = dag.write() {
                let edge = (&gvtx, &pvtx);
                guard.add_edge(edge);
                return Some(genesis.get_hash().clone())
            }
        }
        None
    }

    pub(crate) fn mine_next_convergence_block(dag: &mut MinerDag) -> Option<String> {
        let keypair = Keypair::random();
        let mut miner = create_miner_from_keypair(&keypair);
        miner.dag = dag.clone();
        if let Ok(cblock) = miner.try_mine() {
            if let Block::Convergence { ref block } = cblock.clone() {
                let cvtx: Vertex<Block, String> = cblock.into();
                let mut edges: Vec<(Vertex<Block, String>, Vertex<Block, String>)> = vec![]; 
                if let Ok(guard) = dag.read() {
                    block.clone().get_ref_hashes().iter().for_each(|t| {
                        if let Some(pvtx) = guard.get_vertex(t.clone()) {
                            edges.push((pvtx.clone(), cvtx.clone()));
                        }
                    });
                }
    
                if let Ok(mut guard) = dag.write() {
                    let edges = edges.iter().map(|(source, reference)| {
                        (source, reference)
                    }).collect();
                    guard.extend_from_edges(edges);
                    return Some(block.get_hash())
                }
            }
        }
    
        None
    }

    pub(crate) fn append_proposal_blocks_to_dag(dag: &mut MinerDag, proposals: Vec<ProposalBlock>) {
        let mut edges: Vec<(Vertex<Block, String>, Vertex<Block, String>)> = vec![];
        for block in proposals.iter() {
            let ref_hash = block.ref_block.clone();
            if let Ok(guard) = dag.read() {
                if let Some(cvtx) = guard.get_vertex(ref_hash) {
                    let pblock = Block::Proposal { block: block.clone() };
                    let pvtx: Vertex<Block, String> = pblock.into();
                    let edge = (cvtx.clone(), pvtx.clone());
                    edges.push(edge);
                }
            }
        }

        let edges: Vec<(&Vertex<Block, String>, &Vertex<Block, String>)> = edges.iter().map(|(source, reference)| {
            (source, reference)
        }).collect();

        if let Ok(mut guard) = dag.write() {
            guard.extend_from_edges(edges);
        }
    }
}
