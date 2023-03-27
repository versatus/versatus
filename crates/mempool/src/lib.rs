pub mod error;
pub mod mempool;

// TODO: merge pool w Mempool later on
pub mod pool;
use anyhow::{Context, Result};
use reqwest::StatusCode;

pub use crate::mempool::*;
// use serde_json::{Error as JsonError, json};
// use serde::{Deserialize, Serialize};

pub async fn create_tx_indexer(txn_record: &TxnRecord) -> Result<StatusCode> {
    let url = "http://localhost:3444/transactions"; // TODO: Move to config
    let req_json =
        serde_json::to_string(txn_record).context("Failed to serialize txn_record to json")?;

    let client = reqwest::Client::new();
    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .body(req_json)
        .send()
        .await
        .context("Request error")?;

    if response.status().is_success() {
        Ok(response.status())
    } else {
        Err(anyhow::anyhow!(
            "Unexpected status code: {}",
            response.status()
        ))
    }
}

#[cfg(test)]
mod tests {

    use std::{
        collections::{HashMap, HashSet},
        time::{SystemTime, UNIX_EPOCH},
    };

    use primitives::Signature;
    use rand::{thread_rng, Rng};
    use secp256k1::ecdsa;
    use tokio;
    use vrrb_core::{
        keypair::KeyPair,
        txn::{NewTxnArgs, Txn},
    };

    use crate::mempool::{LeftRightMempool, TxnRecord, TxnStatus};

    fn mock_txn_signature() -> Signature {
        ecdsa::Signature::from_compact(&[
            0xdc, 0x4d, 0xc2, 0x64, 0xa9, 0xfe, 0xf1, 0x7a, 0x3f, 0x25, 0x34, 0x49, 0xcf, 0x8c,
            0x39, 0x7a, 0xb6, 0xf1, 0x6f, 0xb3, 0xd6, 0x3d, 0x86, 0x94, 0x0b, 0x55, 0x86, 0x82,
            0x3d, 0xfd, 0x02, 0xae, 0x3b, 0x46, 0x1b, 0xb4, 0x33, 0x6b, 0x5e, 0xcb, 0xae, 0xfd,
            0x66, 0x27, 0xaa, 0x92, 0x2e, 0xfc, 0x04, 0x8f, 0xec, 0x0c, 0x88, 0x1c, 0x10, 0xc4,
            0xc9, 0x42, 0x8f, 0xca, 0x69, 0xc1, 0x32, 0xa2,
        ])
        .unwrap()
    }

    #[test]
    fn creates_new_lrmempooldb() {
        let lrmpooldb = LeftRightMempool::new();
        assert_eq!(0, lrmpooldb.size());
    }

    #[tokio::test]
    async fn add_a_single_txn() {
        let keypair = KeyPair::random();

        let txn = Txn::new(NewTxnArgs {
            timestamp: 0,
            sender_address: String::from("aaa1"),
            sender_public_key: keypair.get_miner_public_key().clone(),
            receiver_address: String::from("bbb1"),
            token: None,
            amount: 0,
            validators: Some(HashMap::<String, bool>::new()),
            nonce: 0,
            signature: mock_txn_signature(),
        });

        let mut mpooldb = LeftRightMempool::new();
        match mpooldb.add_txn(&txn, TxnStatus::Pending) {
            Ok(_) => {
                std::thread::sleep(std::time::Duration::from_secs(3));
                assert_eq!(1, mpooldb.size());
            },
            Err(_) => {
                panic!("Adding first transaction was unsuccesful !");
            },
        };

        assert_eq!(1, mpooldb.size());
    }

    #[tokio::test]
    async fn add_twice_same_txn() {
        let keypair = KeyPair::random();

        let txn = Txn::new(NewTxnArgs {
            timestamp: 0,
            sender_address: String::from("aaa1"),
            sender_public_key: keypair.get_miner_public_key().clone(),
            receiver_address: String::from("bbb1"),
            token: None,
            amount: 0,
            validators: Some(HashMap::<String, bool>::new()),
            nonce: 0,
            signature: mock_txn_signature(),
        });

        let mut mpooldb = LeftRightMempool::new();

        match mpooldb.add_txn(&txn, TxnStatus::Pending) {
            Ok(_) => {
                assert_eq!(1, mpooldb.size());
            },
            Err(_) => {
                panic!("Adding first transaction was unsuccesful !");
            },
        };

        match mpooldb.add_txn(&txn, TxnStatus::Pending) {
            Ok(_) => {
                assert_eq!(1, mpooldb.size());
            },
            Err(_) => {
                assert_eq!(1, mpooldb.size());
            },
        };

        assert_eq!(1, mpooldb.size());
    }

    #[tokio::test]
    async fn add_two_different_txn() {
        let keypair = KeyPair::random();

        let txn1 = Txn::new(NewTxnArgs {
            timestamp: 0,
            sender_address: String::from("aaa1"),
            sender_public_key: keypair.get_miner_public_key().clone(),
            receiver_address: String::from("bbb1"),
            token: None,
            amount: 0,
            validators: Some(HashMap::<String, bool>::new()),
            nonce: 0,
            signature: mock_txn_signature(),
        });

        let txn2 = Txn::new(NewTxnArgs {
            timestamp: 0,
            sender_address: String::from("aaa1"),
            sender_public_key: keypair.get_miner_public_key().clone(),
            receiver_address: String::from("ccc1"),
            token: None,
            amount: 0,
            validators: Some(HashMap::<String, bool>::new()),
            nonce: 0,
            signature: mock_txn_signature(),
        });

        let mut mpooldb = LeftRightMempool::new();

        match mpooldb.add_txn(&txn1, TxnStatus::Pending) {
            Ok(_) => {
                assert_eq!(1, mpooldb.size());
            },
            Err(_) => {
                panic!("Adding first transaction was unsuccesful !");
            },
        };

        match mpooldb.add_txn(&txn2, TxnStatus::Pending) {
            Ok(_) => {
                assert_eq!(2, mpooldb.size());
            },
            Err(_) => {
                panic!("Adding another, different transaction was unsuccesful !");
            },
        };
    }

    #[tokio::test]
    async fn add_and_retrieve_txn() {
        let keypair = KeyPair::random();

        let sender_address = String::from("aaa1");
        let receiver_address = String::from("bbb1");
        let txn_amount: u128 = 1010101;

        let now = chrono::offset::Utc::now().timestamp();

        let txn = Txn::new(NewTxnArgs {
            timestamp: now,
            sender_address: String::from("aaa1"),
            sender_public_key: keypair.get_miner_public_key().clone(),
            receiver_address: String::from("bbb1"),
            token: None,
            amount: txn_amount,
            validators: Some(HashMap::<String, bool>::new()),
            nonce: 0,
            signature: mock_txn_signature(),
        });

        let txn_id = txn.digest();

        let mut mpooldb = LeftRightMempool::new();
        match mpooldb.add_txn(&txn, TxnStatus::Pending) {
            Ok(_) => {
                assert_eq!(1, mpooldb.size());
            },
            Err(_) => {
                panic!("Adding transaction was unsuccesful !");
            },
        };

        let now = chrono::offset::Utc::now().timestamp();

        // Test single Txn retrieval
        if let Some(txn_retrieved) = mpooldb.get_txn(&txn.digest().clone()) {
            assert_eq!(txn_retrieved.digest(), txn_id);
            assert_eq!(txn_retrieved.timestamp, now);
            assert_eq!(txn_retrieved.sender_address, sender_address);
            assert_eq!(txn_retrieved.receiver_address, receiver_address);
            assert_eq!(txn_retrieved.amount(), txn_amount);
        } else {
            panic!("No transaction found!");
        }

        // Test TxnRecord retrieval
        if let Some(txn_rec_retrieved) = mpooldb.get(&txn.digest()) {
            let txn_retrieved = txn_rec_retrieved.txn;
            assert_eq!(txn_retrieved.digest(), txn_id);
            assert_eq!(txn_retrieved.timestamp, now);
            assert_eq!(txn_retrieved.sender_address, sender_address);
            assert_eq!(txn_retrieved.receiver_address, receiver_address);
            assert_eq!(txn_retrieved.amount(), txn_amount);
        } else {
            panic!("No transaction found!");
        }
    }

    #[tokio::test]
    async fn add_batch_of_transactions() {
        let keypair = KeyPair::random();
        let mut txns = HashSet::<Txn>::new();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let sender_address = String::from("aaa1");
        let receiver_address = String::from("bbb1");
        let txn_amount: u128 = 1010101;

        for n in 1..101 {
            let txn = Txn::new(NewTxnArgs {
                timestamp: 0,
                sender_address: String::from("aaa1"),
                sender_public_key: keypair.get_miner_public_key().clone(),
                receiver_address: receiver_address.clone(),
                token: None,
                amount: txn_amount + n,
                validators: Some(HashMap::<String, bool>::new()),
                nonce: 0,
                signature: mock_txn_signature(),
            });

            txns.insert(txn);
        }

        let mut mpooldb = LeftRightMempool::new();

        match mpooldb.extend(txns.clone()) {
            Ok(_) => {
                assert_eq!(100, mpooldb.size());
            },
            Err(_) => {
                panic!("Adding transaction was unsuccesful !");
            },
        };

        let index = thread_rng().gen_range(0..txns.len());

        let map_values = mpooldb.pool();
        let map_values = map_values
            .values()
            .map(|v| v.to_owned())
            .collect::<Vec<TxnRecord>>();

        let record = map_values.get(index).unwrap().to_owned();

        let txn_id = record.txn_id;
        let test_txn_amount = record.txn.amount();

        if let Some(txn_retrieved) = mpooldb.get_txn(&record.txn.digest()) {
            assert_eq!(txn_retrieved.sender_address, sender_address);
            assert_eq!(txn_retrieved.receiver_address, receiver_address);
            assert_eq!(txn_retrieved.amount(), test_txn_amount);
        } else {
            panic!("No transaction found!");
        }
    }

    #[tokio::test]
    async fn remove_single_txn_by_id() {
        let keypair = KeyPair::random();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let txn1 = Txn::new(NewTxnArgs {
            timestamp: 0,
            sender_address: String::from("aaa1"),
            sender_public_key: keypair.get_miner_public_key().clone(),
            receiver_address: String::from("bbb1"),
            token: None,
            amount: 0,
            validators: Some(HashMap::<String, bool>::new()),
            nonce: 0,
            signature: mock_txn_signature(),
        });

        let txn2 = Txn::new(NewTxnArgs {
            timestamp: 0,
            sender_address: String::from("aaa1"),
            sender_public_key: keypair.get_miner_public_key().clone(),
            receiver_address: String::from("ccc1"),
            token: None,
            amount: 0,
            validators: Some(HashMap::<String, bool>::new()),
            nonce: 0,
            signature: mock_txn_signature(),
        });

        let txn2_id = txn2.digest();

        let mut mpooldb = LeftRightMempool::new();

        match mpooldb.add_txn(&txn1, TxnStatus::Pending) {
            Ok(_) => {
                assert_eq!(1, mpooldb.size());
            },
            Err(_) => {
                panic!("Adding first transaction was unsuccesful !");
            },
        };

        match mpooldb.add_txn(&txn2, TxnStatus::Pending) {
            Ok(_) => {
                assert_eq!(2, mpooldb.size());
            },
            Err(_) => {
                panic!("Adding another, different transaction was unsuccesful !");
            },
        };

        match mpooldb.remove_txn_by_id(&txn2_id) {
            Ok(_) => {
                assert_eq!(1, mpooldb.size());
            },
            Err(_) => {
                panic!("Adding another, different transaction was unsuccesful !");
            },
        };
    }

    #[tokio::test]
    async fn remove_single_txn() {
        let keypair = KeyPair::random();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let txn1 = Txn::new(NewTxnArgs {
            timestamp: 0,
            sender_address: String::from("aaa1"),
            sender_public_key: keypair.get_miner_public_key().clone(),
            receiver_address: String::from("bbb1"),
            token: None,
            amount: 0,
            validators: Some(HashMap::<String, bool>::new()),
            nonce: 0,
            signature: mock_txn_signature(),
        });

        let txn2 = Txn::new(NewTxnArgs {
            timestamp: 0,
            sender_address: String::from("aaa1"),
            sender_public_key: keypair.get_miner_public_key().clone(),
            receiver_address: String::from("ccc1"),
            token: None,
            amount: 0,
            validators: Some(HashMap::<String, bool>::new()),
            nonce: 0,
            signature: mock_txn_signature(),
        });

        let mut mpooldb = LeftRightMempool::new();

        match mpooldb.add_txn(&txn1, TxnStatus::Pending) {
            Ok(_) => {
                assert_eq!(1, mpooldb.size());
            },
            Err(_) => {
                panic!("Adding first transaction was unsuccesful !");
            },
        };

        match mpooldb.add_txn(&txn2, TxnStatus::Pending) {
            Ok(_) => {
                assert_eq!(2, mpooldb.size());
            },
            Err(_) => {
                panic!("Adding another, different transaction was unsuccesful !");
            },
        };

        match mpooldb.remove_txn(&txn1, TxnStatus::Pending) {
            Ok(_) => {
                assert_eq!(1, mpooldb.size());
            },
            Err(_) => {
                panic!("Adding another, different transaction was unsuccesful !");
            },
        };
    }

    #[test]
    fn remove_txn_batch() {
        let keypair = KeyPair::random();
        let mut txns = HashSet::<Txn>::new();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        // let txn_id = String::from("1");
        let sender_address = String::from("aaa1");
        let receiver_address = String::from("bbb1");
        let txn_amount: u128 = 1010101;

        for n in 1..101 {
            let txn = Txn::new(NewTxnArgs {
                timestamp: 0,
                sender_address: String::from("aaa1"),
                sender_public_key: keypair.get_miner_public_key().clone(),
                receiver_address: receiver_address.clone(),
                token: None,
                amount: txn_amount + n,
                validators: Some(HashMap::<String, bool>::new()),
                nonce: 0,
                signature: mock_txn_signature(),
            });

            txns.insert(txn);
        }

        let mut mpooldb = LeftRightMempool::new();
        match mpooldb.add_txn_batch(&txns, TxnStatus::Pending) {
            Ok(_) => {
                assert_eq!(100, mpooldb.size());
            },
            Err(_) => {
                panic!("Adding transactions was unsuccesful !");
            },
        };
        match mpooldb.remove_txn_batch(&txns, TxnStatus::Pending) {
            Ok(_) => {
                assert_eq!(0, mpooldb.size());
            },
            Err(_) => {
                panic!("Removing transactions was unsuccesful !");
            },
        };
    }

    #[test]
    fn batch_write_and_parallel_reads() {
        let keypair = KeyPair::random();
        let txn_id_max = 11;
        let mut lrmpooldb = LeftRightMempool::new();
        let mut txns = HashSet::<Txn>::new();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let sender_address = String::from("aaa1");
        let receiver_address = String::from("bbb1");
        let txn_amount: u128 = 1010101;

        for n in 1..u128::try_from(txn_id_max).unwrap_or(0) {
            let txn = Txn::new(NewTxnArgs {
                timestamp: 0,
                sender_address: String::from("aaa1"),
                sender_public_key: keypair.get_miner_public_key().clone(),
                receiver_address: receiver_address.clone(),
                token: None,
                amount: txn_amount + n,
                validators: Some(HashMap::<String, bool>::new()),
                nonce: 0,
                signature: mock_txn_signature(),
            });

            txns.insert(txn);
        }

        match lrmpooldb.add_txn_batch(&txns, TxnStatus::Pending) {
            Ok(_) => {
                assert_eq!(txn_id_max - 1, lrmpooldb.size());
            },
            Err(_) => {
                panic!("Adding transactions was unsuccesful !");
            },
        };

        [0..txn_id_max]
            .iter()
            .map(|_| {
                let mpool_hdl = lrmpooldb.factory();

                std::thread::spawn(move || {
                    let read_hdl = mpool_hdl.handle();
                    assert_eq!(read_hdl.len(), txn_id_max - 1);
                })
            })
            .for_each(|handle| {
                handle.join().unwrap();
            });
    }
}
