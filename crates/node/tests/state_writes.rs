#![cfg(test)]

use std::{
    collections::HashSet,
    net::SocketAddr,
    sync::{Arc, RwLock},
};

use block::{Block, BlockHash, GenesisBlock, InnerBlock, ProposalBlock};
use bulldag::{graph::BullDag, vertex::Vertex};
use hbbft::crypto::SecretKeyShare;
use node::state_module::{StateModule, StateModuleConfig};
use primitives::{generate_account_keypair, Address};
use secp256k1::{Message, PublicKey, SecretKey};
use storage::vrrbdb::{VrrbDb, VrrbDbConfig};
use tokio::sync::mpsc::channel;
use vrrb_core::{
    account::Account,
    claim::Claim,
    keypair::Keypair,
    txn::{generate_txn_digest_vec, NewTxnArgs, TransactionDigest, Txn},
};

pub type StateDag = Arc<RwLock<BullDag<Block, BlockHash>>>;

#[tokio::test]
async fn vrrbdb_should_update_with_new_block() {
    let db_config = VrrbDbConfig::default();
    let db = VrrbDb::new(db_config.clone());
    let accounts: Vec<(Address, Account)> = produce_accounts(5);
    let dag: StateDag = Arc::new(RwLock::new(BullDag::new()));
    let (events_tx, _) = channel(100);
    let config = StateModuleConfig {
        db,
        events_tx,
        dag: dag.clone(),
    };
    let mut state_module = StateModule::new(config);
    let state_res = state_module.extend_accounts(accounts.clone());
    let genesis = produce_genesis_block();

    assert!(state_res.is_ok());

    let gblock: Block = genesis.clone().into();
    let gvtx: Vertex<Block, BlockHash> = gblock.into();
    if let Ok(mut guard) = dag.write() {
        guard.add_vertex(&gvtx);
    }

    let proposals = produce_proposal_blocks(genesis.clone().hash, accounts.clone(), 5, 5);

    let edges: Vec<(Vertex<Block, BlockHash>, Vertex<Block, BlockHash>)> = {
        proposals
            .into_iter()
            .map(|pblock| {
                let pblock: Block = pblock.clone().into();
                let pvtx: Vertex<Block, BlockHash> = pblock.into();
                (gvtx.clone(), pvtx.clone())
            })
            .collect()
    };

    if let Ok(mut guard) = dag.write() {
        edges
            .iter()
            .for_each(|(source, reference)| guard.add_edge((&source, &reference)));
    }

    if let Some(block_hash) = produce_convergence_block(dag.clone()) {
        let _ = state_module.update_state(block_hash);
    }

    state_module.commit();

    let handle = state_module.read_handle();
    let store = handle.state_store_values();

    accounts.iter().for_each(|(address, _)| {
        let acct_opt = store.get(address);
        assert!(acct_opt.is_some());
        if let Some(account) = store.get(address) {
            println!("{:?}", account);

            let digests = account.digests.clone();

            assert!(!digests.get_sent().is_empty());
            assert!(!digests.get_recv().is_empty());
            assert!(digests.get_stake().is_empty());
        }
    });
}

fn produce_accounts(n: usize) -> Vec<(Address, Account)> {
    (0..n)
        .into_iter()
        .map(|_| {
            let kp = generate_account_keypair();
            let mut account = Account::new(kp.1.clone());
            account.credits = 1_000_000_000_000_000_000_000_000_000u128;
            (Address::new(kp.1.clone()), account.clone())
        })
        .collect()
}

fn produce_random_claims(n: usize) -> HashSet<Claim> {
    (0..n)
        .into_iter()
        .map(|_| {
            let kp = Keypair::random();
            let address = Address::new(kp.miner_kp.1.clone());
            let ip_address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
            let signature = Claim::signature_for_valid_claim(
                kp.miner_kp.1.clone(),
                ip_address.clone(),
                kp.get_miner_secret_key().secret_bytes().to_vec(),
            )
            .unwrap();

            Claim::new(
                kp.miner_kp.1.clone(),
                address.clone(),
                ip_address,
                signature,
            )
            .unwrap()
        })
        .collect()
}

fn produce_random_txs(accounts: &Vec<(Address, Account)>) -> HashSet<Txn> {
    accounts
        .clone()
        .iter()
        .enumerate()
        .map(|(idx, (address, account))| {
            let receiver: (Address, Account);
            if (idx + 1) == accounts.len() {
                receiver = accounts[0].clone();
            } else {
                receiver = accounts[idx + 1].clone();
            }

            let mut validators: Vec<(String, bool)> = vec![];

            accounts.clone().iter().for_each(|validator| {
                if (validator.clone() != receiver)
                    && (validator.clone() != (address.clone(), account.clone()))
                {
                    let pk = validator.clone().0.public_key().to_string();
                    validators.push((pk, true));
                }
            });
            create_txn_from_accounts((address.clone(), account.clone()), receiver.0, validators)
        })
        .collect()
}

fn produce_genesis_block() -> GenesisBlock {
    let genesis = miner::test_helpers::mine_genesis();
    genesis.unwrap()
}

fn produce_proposal_blocks(
    last_block_hash: BlockHash,
    accounts: Vec<(Address, Account)>,
    n: usize,
    ntx: usize,
) -> Vec<ProposalBlock> {
    (0..n)
        .into_iter()
        .map(|_| {
            let kp = Keypair::random();
            let address = Address::new(kp.miner_kp.1.clone());
            let ip_address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
            let signature = Claim::signature_for_valid_claim(
                kp.miner_kp.1.clone(),
                ip_address.clone(),
                kp.get_miner_secret_key().secret_bytes().to_vec(),
            )
            .unwrap();
            let from = Claim::new(
                kp.miner_kp.1.clone(),
                address.clone(),
                ip_address,
                signature,
            )
            .unwrap();
            let txs = produce_random_txs(&accounts);
            let claims = produce_random_claims(ntx);
            let txn_list = txs.into_iter().map(|tx| (tx.clone().into(), tx)).collect();
            let claim_list = claims
                .into_iter()
                .map(|claim| (claim.hash, claim))
                .collect();
            ProposalBlock::build(
                last_block_hash.clone(),
                0,
                0,
                txn_list,
                claim_list,
                from,
                SecretKeyShare::default(),
            )
        })
        .collect()
}

fn produce_convergence_block(dag: Arc<RwLock<BullDag<Block, BlockHash>>>) -> Option<BlockHash> {
    let keypair = Keypair::random();
    let mut miner = miner::test_helpers::create_miner_from_keypair(&keypair);
    miner.dag = dag.clone();
    let last_block = miner::test_helpers::get_genesis_block_from_dag(dag.clone());

    if let Some(block) = last_block {
        miner.last_block = Some(Arc::new(block));
    }

    if let Ok(cblock) = miner.try_mine() {
        if let Block::Convergence { ref block } = cblock.clone() {
            let cvtx: Vertex<Block, String> = cblock.into();
            let mut edges: Vec<(Vertex<Block, String>, Vertex<Block, String>)> = vec![];
            if let Ok(guard) = dag.clone().read() {
                block.clone().get_ref_hashes().iter().for_each(|t| {
                    if let Some(pvtx) = guard.get_vertex(t.clone()) {
                        edges.push((pvtx.clone(), cvtx.clone()));
                    }
                });
            }

            if let Ok(mut guard) = dag.clone().write() {
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

pub fn create_keypair() -> (SecretKey, PublicKey) {
    let kp = Keypair::random();
    kp.miner_kp
}

pub(crate) fn create_txn_from_accounts(
    sender: (Address, Account),
    receiver: Address,
    validators: Vec<(String, bool)>,
) -> Txn {
    let (sk, pk) = create_keypair();
    let saddr = sender.0.clone();
    let raddr = receiver.clone();
    let amount = 100u128.pow(2);
    let token = None;

    let mut validators = validators
        .iter()
        .map(|(k, v)| (k.to_string().clone(), v.clone()))
        .collect();

    let txn_args = NewTxnArgs {
        timestamp: 0,
        sender_address: saddr,
        sender_public_key: pk.clone(),
        receiver_address: raddr,
        token,
        amount,
        signature: sk
            .sign_ecdsa(Message::from_hashed_data::<secp256k1::hashes::sha256::Hash>(b"vrrb")),
        validators: Some(validators),
        nonce: sender.1.nonce + 1,
    };

    let mut txn = Txn::new(txn_args);

    txn.sign(&sk);

    let txn_digest_vec = generate_txn_digest_vec(
        txn.timestamp,
        txn.sender_address.to_string(),
        txn.sender_public_key.clone(),
        txn.receiver_address.to_string(),
        txn.token.clone(),
        txn.amount,
        txn.nonce,
    );

    let _digest = TransactionDigest::from(txn_digest_vec);

    txn
}
