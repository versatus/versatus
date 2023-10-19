use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
};

use block::{Block, GenesisBlock, InnerBlock, ProposalBlock};
use bulldag::{graph::BullDag, vertex::Vertex};
use ethereum_types::U256;
use primitives::{Address, NodeId, PublicKey, SecretKey, Signature};
use ritelinked::LinkedHashMap;
use secp256k1::Message;
use sha2::Digest;
use vrrb_core::transactions::{
    generate_transfer_digest_vec, Transaction, TransactionDigest, TransactionKind,
};
use vrrb_core::{claim::Claim, keypair::Keypair};

use crate::{result::MinerError, Miner, MinerConfig};

/// Move this into primitives and call it simply `BlockDag`
pub type MinerDag = Arc<RwLock<BullDag<Block, String>>>;

/// Helper function to create a random Miner.
pub fn create_miner() -> Miner {
    let (secret_key, public_key) = create_keypair();
    let dag: MinerDag = Arc::new(RwLock::new(BullDag::new()));
    let ip_address = "127.0.0.1:8080".parse().unwrap();

    let signature =
        Claim::signature_for_valid_claim(public_key, ip_address, secret_key.secret_bytes().into())
            .unwrap();

    let claim = Claim::new(
        public_key,
        Address::new(public_key),
        ip_address,
        signature,
        String::from("test-miner-node"),
    )
    .unwrap();

    let config = MinerConfig {
        secret_key,
        public_key,
        ip_address,
        dag,
        claim,
    };
    Miner::new(config, NodeId::default()).unwrap()
}

/// Helper function to create a miner from a `Keypair`
pub fn create_miner_from_keypair(kp: &Keypair) -> Miner {
    let (secret_key, public_key) = kp.miner_kp;
    let dag: MinerDag = Arc::new(RwLock::new(BullDag::new()));
    let ip_address = "127.0.0.1:8080".parse().unwrap();
    let signature =
        Claim::signature_for_valid_claim(public_key, ip_address, secret_key.secret_bytes().into())
            .unwrap();

    let claim = Claim::new(
        public_key,
        Address::new(public_key),
        ip_address,
        signature,
        String::from("test-miner-node"),
    )
    .unwrap();

    let config = MinerConfig {
        secret_key,
        ip_address,
        public_key,
        claim,
        dag,
    };
    Miner::new(config, NodeId::default()).unwrap()
}

pub fn create_miner_from_keypair_return_dag(kp: &Keypair) -> (Miner, MinerDag) {
    let miner = create_miner_from_keypair(kp);
    (miner.clone(), miner.dag)
}

pub fn create_miner_from_keypair_and_dag(kp: &Keypair, dag: MinerDag) -> Miner {
    let mut miner = create_miner_from_keypair(kp);
    miner.dag = dag;
    miner
}

/// Helper function to create a `MinerKeypair` which is
/// simply `(SecretKey, PublicKey)`
pub fn create_keypair() -> (SecretKey, PublicKey) {
    let kp = Keypair::random();
    kp.miner_kp
}

/// Helper function to create an address from a `&PublicKey`
pub fn create_address(public_key: &PublicKey) -> Address {
    Address::new(*public_key)
}

/// Helper function to create a claim from a `&PublicKey` and `&Address` and
/// `ip_address` and `signature`
pub fn create_claim(
    pk: &PublicKey,
    addr: &Address,
    ip_address: SocketAddr,
    signature: String,
) -> Claim {
    Claim::new(*pk, addr.clone(), ip_address, signature, NodeId::default()).unwrap()
}

/// Helper function to create a random message and signature
/// returning `(Message, Keypair, Signature)`
pub fn create_and_sign_message() -> (Message, Keypair, Signature) {
    let kp = Keypair::random();
    let message = b"Test Message";
    let msg = {
        let mut hasher = sha2::Sha256::new();
        hasher.update(message);
        let message = hasher.finalize();
        Message::from_slice(&message[..]).unwrap()
    };

    let sig = kp.miner_kp.0.sign_ecdsa(msg);

    (msg, kp, sig)
}

/// Helper function to mine a `GenesisBlock` and
/// return an `Option<GenesisBlock>`
/// This is currently using a deprecated method
/// `miner.mine_genesis_block` will be removed soon
/// and replaced by a different method.
pub fn mine_genesis() -> Option<GenesisBlock> {
    let miner = create_miner();

    let claim = miner.generate_claim().unwrap();

    let claim_list = { vec![(claim.hash, claim)].iter().cloned().collect() };

    miner.mine_genesis_block(claim_list)
}

/// Helper function to create `n` number of `Txn` and
/// return an `Iterator` of `(TransactionDigest, Txn)`
/// to be collected by the caller.
pub(crate) fn create_txns(n: usize) -> impl Iterator<Item = (TransactionDigest, TransactionKind)> {
    (0..n).map(|n| {
        let (sk, pk) = create_keypair();
        let (_, rpk) = create_keypair();
        let saddr = create_address(&pk);
        let raddr = create_address(&rpk);
        let amount = (n.pow(2)) as u128;

        let mut txn =
            TransactionKind::transfer_builder()
                .timestamp(0)
                .sender_address(saddr)
                .sender_public_key(pk)
                .receiver_address(raddr)
                .amount(amount)
                .signature(sk.sign_ecdsa(Message::from_hashed_data::<
                    secp256k1::hashes::sha256::Hash,
                >(b"vrrb")))
                .nonce(n as u128)
                .build_kind()
                .expect("Failed to build transaction");

        txn.sign(&sk);

        let txn_digest_vec = generate_transfer_digest_vec(
            txn.timestamp(),
            txn.sender_address().to_string(),
            txn.sender_public_key(),
            txn.receiver_address().to_string(),
            txn.token(),
            txn.amount(),
            txn.nonce(),
        );

        let digest = TransactionDigest::from(txn_digest_vec);
        (digest, txn)
    })
}

/// Helper function to create `n` number of `Claim`s and
/// return an `Iterator` of `(String, Claim)` to be collected
/// by the caller
pub fn create_claims(n: usize) -> impl Iterator<Item = (U256, Claim)> {
    (0..n).map(|_| {
        let (sk, pk) = create_keypair();
        let addr = create_address(&pk);
        let ip_address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
        let signature =
            Claim::signature_for_valid_claim(pk, ip_address, sk.secret_bytes().to_vec()).unwrap();
        let claim = create_claim(&pk, &addr, ip_address, signature);
        (claim.hash, claim)
    })
}

/// A helper function to attempt to mine a `ConvergenceBlock`
/// with a random `miner`
pub fn mine_convergence_block() -> Result<Block, MinerError> {
    let mut miner = create_miner();
    miner.try_mine()
}

/// A helper function to attempt to mine a `ConvergenceBlock`
/// that signals a change in `Epoch` i.e. a block
/// with a `round % Epoch == 0`
pub fn mine_convergence_block_epoch_change() -> Result<Block, MinerError> {
    let mut miner = create_miner();
    //TODO: Add Mock Convergence Block with round height of 29.999999mm
    miner.try_mine()
}

/// A helper function that creates a `Miner` and returns both the
/// `Miner` and the `MinerDag`
pub fn create_miner_return_dag() -> (Miner, MinerDag) {
    let miner = create_miner();
    let dag = miner.dag.clone();

    (miner, dag)
}

/// A helper function that creates a random `Miner` and provides
/// an existing `MinerDag` to replace the default one in the
/// `Miner`. Returns both the `Miner` and the `MinerDag`
pub fn create_miner_from_dag(dag: &MinerDag) -> (Miner, MinerDag) {
    let mut miner = create_miner();
    miner.dag = dag.clone();

    (miner, dag.clone())
}

/// A helper function to build a single `ProposalBlock` and return it.
pub fn build_single_proposal_block(
    last_block_hash: String,
    n_txns: usize,
    n_claims: usize,
    round: u128,
    epoch: u128,
    from: Claim,
    sk: signer::engine::SignerEngine,
) -> ProposalBlock {
    let txns = create_txns(n_txns).collect();
    let claims = create_claims(n_claims).collect();
    ProposalBlock::build(last_block_hash, round, epoch, txns, claims, from, sk)
}

/// A helper function to build `n` number of porposal blocks
/// from random `Claim`s and return a `Vec<ProposalBlock>`
pub fn build_multiple_proposal_blocks_single_round(
    n_blocks: usize,
    last_block_hash: String,
    n_txns: usize,
    n_claims: usize,
    round: u128,
    epoch: u128,
    sk: signer::engine::SignerEngine,
) -> Vec<ProposalBlock> {
    (0..n_blocks)
        .map(|_| {
            let keypair = Keypair::random();
            let (_, public_key) = keypair.miner_kp;
            let address = Address::new(public_key);
            let ip_address: SocketAddr = "127.0.0.1:8080".parse().unwrap();
            let signature = Claim::signature_for_valid_claim(
                public_key,
                ip_address,
                keypair.get_miner_secret_key().secret_bytes().to_vec(),
            )
            .unwrap();
            let claim = Claim::new(
                public_key,
                address,
                ip_address,
                signature,
                NodeId::default(),
            )
            .unwrap();
            build_single_proposal_block(
                last_block_hash.clone(),
                n_txns,
                n_claims,
                round,
                epoch,
                claim,
                sk.clone(),
            )
        })
        .collect()
}

/// A recursive helper function that takes in a mutable
/// `MinerDag` and some information regarding the number
/// of rounds, number of blocks (`ProposalBlock`) per round
/// The current round (as a mutable reference), and the epoch,
/// as well as the `last_block_hash` which is either
/// the hash of the `GenesisBlock` or a hash of the most recent
/// `ConvergenceBlock`
///
/// The function checks whether the current `round` that it is
/// building is less than the number of rounds (`n_rounds`) the
/// caller is asking for.
///
/// If so, then it stops, otherwise it proceeds with the following logic:
///     
/// Check if the DAG has a GenesisBlock.
///
/// If so:
///   - Mine a ConvergenceBlock and append it to the MinerDag
///   referencing the previous round ProposalBlocks
///
///   - Add 1 to the round
///
///   - Build ProposalBlocks that reference the new ConvergenceBlock.
///
///   - Append the new ProposalBlocks to the DAG referencing
///   the most recent ConvergenceBlock.
///
///   - Recursively calls itself passing in the most recent
///   ConvergenceBlock hash as the `last_block_hash` and the
///   updated round, as well as the rest of the information.
///
/// Otherwise:
///   - Add a genesis block, and a single, random, empty ProposalBlock
///   to the DAG as the root vertex and first leaf the two make
///   up the first edge.
///
///   - Add 1 to the round
///
///   - Recursively calls itself passing in the GenesisBlock hash
///   as the last_block_hash and the updated round as the
///   round, as well as all the other data.
#[allow(clippy::too_many_arguments)]
pub fn build_multiple_rounds(
    dag: MinerDag,
    n_blocks: usize,
    n_txns: usize,
    n_claims: usize,
    n_rounds: usize,
    round: &mut usize,
    epoch: usize,
    sk: signer::engine::SignerEngine,
) {
    if n_rounds > *round {
        if dag_has_genesis(dag.clone()) {
            if let Some(last_block_hash) = mine_next_convergence_block(dag.clone()) {
                *round += 1usize;
                let proposals = build_multiple_proposal_blocks_single_round(
                    n_blocks,
                    last_block_hash,
                    n_txns,
                    n_claims,
                    *round as u128,
                    epoch as u128,
                    sk.clone(),
                );

                append_proposal_blocks_to_dag(&mut dag.clone(), proposals);
                build_multiple_rounds(
                    dag,
                    n_blocks,
                    n_txns,
                    n_claims,
                    n_rounds,
                    round,
                    epoch,
                    sk.clone(),
                );
            };
        } else if add_genesis_to_dag(&mut dag.clone()).is_some() {
            *round += 1usize;
            build_multiple_rounds(
                dag,
                n_blocks,
                n_txns,
                n_claims,
                n_rounds,
                round,
                epoch,
                sk.clone(),
            );
        }
    }
}

/// Checks whether the DAG already has a root vertex
/// returns true if so, false if not
pub fn dag_has_genesis(dag: MinerDag) -> bool {
    dag.read().unwrap().len() > 0
}

/// build and adds a `GenesisBlock` to the `MinerDag`
/// returns the `Some(hash)` if successful otherwise returns None
pub fn add_genesis_to_dag(dag: &mut MinerDag) -> Option<String> {
    let mut prop_vertices = Vec::new();
    let genesis = mine_genesis();
    let keypair = Keypair::random();
    let signer = signer::engine::SignerEngine::new(
        *keypair.get_miner_public_key(),
        *keypair.get_miner_secret_key(),
    );
    let miner = create_miner_from_keypair(&keypair);

    if let Some(genesis) = genesis {
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
            miner.claim,
            signer,
        );
        let pblock = Block::Proposal { block: prop1 };
        let pvtx: Vertex<Block, String> = pblock.into();
        prop_vertices.push(pvtx.clone());
        if let Ok(mut guard) = dag.clone().write() {
            let edge = (&gvtx, &pvtx);
            guard.add_edge(edge);
            return Some(genesis.get_hash());
        }
    }
    None
}

/// Mines the next `ConvergenceBlock` in the `MinerDag`
/// Returns `Some(hash)` if successful otherwise returns `None`
pub fn mine_next_convergence_block(dag: MinerDag) -> Option<String> {
    let keypair = Keypair::random();
    let mut miner = create_miner_from_keypair(&keypair);
    miner.dag = dag.clone();
    let last_block = get_genesis_block_from_dag(dag.clone());

    if let Some(block) = last_block {
        miner.last_block = Some(Arc::new(block));
    }

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
                let edges = edges
                    .iter()
                    .map(|(source, reference)| (source, reference))
                    .collect();
                guard.extend_from_edges(edges);
                return Some(block.get_hash());
            }
        }
    }

    None
}

/// Appends `ProposalBlock`s to the `MinerDag`
pub fn append_proposal_blocks_to_dag(dag: &mut MinerDag, proposals: Vec<ProposalBlock>) {
    let mut edges: Vec<(Vertex<Block, String>, Vertex<Block, String>)> = vec![];
    for block in proposals.iter() {
        let ref_hash = block.ref_block.clone();
        if let Ok(guard) = dag.clone().read() {
            if let Some(cvtx) = guard.get_vertex(ref_hash) {
                let pblock = Block::Proposal {
                    block: block.clone(),
                };
                let pvtx: Vertex<Block, String> = pblock.into();
                let edge = (cvtx.clone(), pvtx.clone());
                edges.push(edge);
            }
        }
    }

    let edges = edges
        .iter()
        .map(|(source, reference)| (source, reference))
        .collect();

    if let Ok(mut guard) = dag.clone().write() {
        guard.extend_from_edges(edges);
    }
}

/// Builds 2 `ProposalBlock`s which contain 5 of the same `Txn`s
/// this is used to test conflict resolution mechanism of the `Miner`
pub fn build_conflicting_proposal_blocks(
    last_block_hash: String,
    round: u128,
    epoch: u128,
) -> (ProposalBlock, ProposalBlock) {
    let txns: LinkedHashMap<TransactionDigest, TransactionKind> = create_txns(5).collect();
    let prop1 =
        build_single_proposal_block_from_txns(last_block_hash.clone(), txns.clone(), round, epoch);

    let prop2 = build_single_proposal_block_from_txns(last_block_hash, txns, round, epoch);

    (prop1, prop2)
}

/// Builds a single `ProposalBlock` and extends the `TxnList` of the
/// `ProposalBlock` with transactions provided in the function call.
pub fn build_single_proposal_block_from_txns(
    last_block_hash: String,
    txns: impl IntoIterator<Item = (TransactionDigest, TransactionKind)>,
    round: u128,
    epoch: u128,
) -> ProposalBlock {
    let kp = Keypair::random();
    let miner = create_miner_from_keypair(&kp);
    let engine =
        signer::engine::SignerEngine::new(*kp.get_miner_public_key(), *kp.get_miner_secret_key());
    let mut prop =
        build_single_proposal_block(last_block_hash, 5, 4, round, epoch, miner.claim, engine);

    prop.txns.extend(txns);

    prop
}

pub fn get_genesis_block_from_dag(dag: MinerDag) -> Option<GenesisBlock> {
    let last_block = {
        if let Ok(guard) = dag.read() {
            let root = guard.get_roots();
            let mut root_iter = root.iter();
            if let Some(idx) = root_iter.next() {
                let last_block = guard.get_vertex(idx.clone());
                if let Some(vtx) = last_block {
                    let gblock = vtx.get_data();
                    if let Block::Genesis { block } = gblock {
                        Some(block)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    };

    last_block
}

pub fn add_orphaned_block_to_dag(
    dag: MinerDag,
    last_block_hash: String,
    txns: impl IntoIterator<Item = (TransactionDigest, TransactionKind)>,
    round: u128,
    epoch: u128,
) {
    let proposal =
        build_single_proposal_block_from_txns(last_block_hash.clone(), txns, round, epoch);

    if let Ok(guard) = dag.read() {
        let vtx_opt = guard.get_vertex(last_block_hash);
        if let Some(vtx) = vtx_opt {
            if let Ok(mut wguard) = dag.write() {
                let pblock = Block::Proposal { block: proposal };
                let pvtx = pblock.into();
                let edge = (vtx, &pvtx);
                wguard.add_edge(edge);
            }
        }
    }
}
