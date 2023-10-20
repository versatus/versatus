pub mod error;
pub mod mempool;

use anyhow::Context;
use reqwest::StatusCode;

pub use crate::mempool::*;

pub async fn create_tx_indexer(txn_record: &TxnRecord) -> anyhow::Result<StatusCode> {
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

    use std::collections::{HashMap, HashSet};

    use primitives::{Address, Signature};
    use rand::{thread_rng, Rng};
    use secp256k1::ecdsa;
    use tokio;
    use vrrb_core::keypair::KeyPair;
    use vrrb_core::transactions::{Transaction, TransactionKind};

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
        let recv_keypair = KeyPair::random();
        let recv_address = Address::new(recv_keypair.get_miner_public_key().clone());

        let txn = TransactionKind::transfer_builder()
            .timestamp(0)
            .sender_address(Address::new(keypair.get_miner_public_key().clone()))
            .sender_public_key(keypair.get_miner_public_key().clone())
            .receiver_address(recv_address)
            .amount(0)
            .validators(HashMap::<String, bool>::new())
            .nonce(0)
            .signature(mock_txn_signature())
            .build_kind()
            .expect("Failed to build transaction");

        let mut mpooldb = LeftRightMempool::new();
        match mpooldb.insert(txn) {
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
        let recv_keypair = KeyPair::random();

        let txn = TransactionKind::transfer_builder()
            .timestamp(0)
            .sender_address(Address::new(keypair.get_miner_public_key().clone()))
            .sender_public_key(keypair.get_miner_public_key().clone())
            .receiver_address(Address::new(recv_keypair.get_miner_public_key().clone()))
            .amount(0)
            .validators(HashMap::<String, bool>::new())
            .nonce(0)
            .signature(mock_txn_signature())
            .build_kind()
            .expect("Failed to build transaction");

        let mut mpooldb = LeftRightMempool::new();

        match mpooldb.insert(txn.clone()) {
            Ok(_) => {
                assert_eq!(1, mpooldb.size());
            },
            Err(_) => {
                panic!("Adding first transaction was unsuccesful !");
            },
        };

        match mpooldb.insert(txn) {
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
        let recv1_keypair = KeyPair::random();
        let recv2_keypair = KeyPair::random();

        let transfer_builder = TransactionKind::transfer_builder()
            .timestamp(0)
            .sender_address(Address::new(keypair.get_miner_public_key().clone()))
            .sender_public_key(keypair.get_miner_public_key().clone())
            .amount(0)
            .validators(HashMap::<String, bool>::new())
            .nonce(0)
            .signature(mock_txn_signature());

        let txn1 = transfer_builder
            .clone()
            .receiver_address(Address::new(recv1_keypair.get_miner_public_key().clone()))
            .build_kind()
            .expect("Failed to build transaction");

        let txn2 = transfer_builder
            .clone()
            .receiver_address(Address::new(recv2_keypair.get_miner_public_key().clone()))
            .build_kind()
            .expect("Failed to build transaction");

        let mut mpooldb = LeftRightMempool::new();

        match mpooldb.insert(txn1) {
            Ok(_) => {
                assert_eq!(1, mpooldb.size());
            },
            Err(_) => {
                panic!("Adding first transaction was unsuccesful !");
            },
        };

        match mpooldb.insert(txn2) {
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
        let recv_keypair = KeyPair::random();

        let sender_address = Address::new(keypair.get_miner_public_key().clone());
        let receiver_address = Address::new(recv_keypair.get_miner_public_key().clone());
        let txn_amount: u128 = 1010101;

        let now = chrono::offset::Utc::now().timestamp();

        let txn = TransactionKind::transfer_builder()
            .timestamp(now)
            .sender_address(sender_address.clone())
            .sender_public_key(keypair.get_miner_public_key().clone())
            .receiver_address(receiver_address.clone())
            .amount(txn_amount)
            .validators(HashMap::<String, bool>::new())
            .nonce(0)
            .signature(mock_txn_signature())
            .build_kind()
            .expect("Failed to build transaction");

        let txn_id = txn.digest();

        let mut mpooldb = LeftRightMempool::new();
        match mpooldb.insert(txn.clone()) {
            Ok(_) => {
                assert_eq!(1, mpooldb.size());
            },
            Err(_) => {
                panic!("Adding transaction was unsuccesful !");
            },
        };

        let now = chrono::offset::Utc::now().timestamp();
        let delta = 10i64;
        // Test single Txn retrieval
        if let Some(txn_retrieved) = mpooldb.get_txn(&txn.digest().clone()) {
            assert_eq!(txn_retrieved.digest(), txn_id);
            assert!((txn_retrieved.timestamp() - now) <= delta);
            assert_eq!(txn_retrieved.sender_address(), sender_address);
            assert_eq!(txn_retrieved.receiver_address(), receiver_address);
            assert_eq!(txn_retrieved.amount(), txn_amount);
        } else {
            panic!("No transaction found!");
        }

        // Test TxnRecord retrieval
        if let Some(txn_rec_retrieved) = mpooldb.get(&txn.digest()) {
            let txn_retrieved = txn_rec_retrieved.txn;
            assert_eq!(txn_retrieved.digest(), txn_id);
            assert!((txn_retrieved.timestamp() - now) <= delta);
            assert_eq!(txn_retrieved.sender_address(), sender_address);
            assert_eq!(txn_retrieved.receiver_address(), receiver_address);
            assert_eq!(txn_retrieved.amount(), txn_amount);
        } else {
            panic!("No transaction found!");
        }
    }

    #[tokio::test]
    async fn add_batch_of_transactions() {
        let keypair = KeyPair::random();
        let recv_keypair = KeyPair::random();

        let mut txns = HashSet::<TransactionKind>::new();

        let sender_address = Address::new(keypair.get_miner_public_key().clone());
        let receiver_address = Address::new(recv_keypair.get_miner_public_key().clone());
        let txn_amount: u128 = 1010101;

        let transfer_builder = TransactionKind::transfer_builder()
            .timestamp(0)
            .sender_address(sender_address.clone())
            .sender_public_key(keypair.get_miner_public_key().clone())
            .receiver_address(receiver_address.clone())
            .validators(HashMap::<String, bool>::new())
            .nonce(0)
            .signature(mock_txn_signature());

        for n in 1..101 {
            let txn = transfer_builder
                .clone()
                .amount(txn_amount + n)
                .build_kind()
                .expect("Failed to build transaction");

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

        let test_txn_amount = record.txn.amount();

        if let Some(txn_retrieved) = mpooldb.get_txn(&record.txn.digest()) {
            assert_eq!(txn_retrieved.sender_address(), sender_address);
            assert_eq!(txn_retrieved.receiver_address(), receiver_address);
            assert_eq!(txn_retrieved.amount(), test_txn_amount);
        } else {
            panic!("No transaction found!");
        }
    }

    #[tokio::test]
    async fn remove_single_txn_by_id() {
        let keypair = KeyPair::random();
        let recv1_keypair = KeyPair::random();
        let recv2_keypair = KeyPair::random();

        let sender_address = Address::new(keypair.get_miner_public_key().clone());
        let recv1_address = Address::new(recv1_keypair.get_miner_public_key().clone());
        let recv2_address = Address::new(recv2_keypair.get_miner_public_key().clone());

        let txn1 = TransactionKind::transfer_builder()
            .timestamp(0)
            .sender_address(sender_address.clone())
            .sender_public_key(keypair.get_miner_public_key().clone())
            .receiver_address(recv1_address.clone())
            .amount(0)
            .validators(HashMap::<String, bool>::new())
            .nonce(0)
            .signature(mock_txn_signature())
            .build_kind()
            .expect("Failed to build transaction");

        let txn2 = TransactionKind::transfer_builder()
            .timestamp(0)
            .sender_address(sender_address.clone())
            .sender_public_key(keypair.get_miner_public_key().clone())
            .receiver_address(recv2_address.clone())
            .amount(0)
            .validators(HashMap::<String, bool>::new())
            .nonce(0)
            .signature(mock_txn_signature())
            .build_kind()
            .expect("Failed to build transaction");

        let txn2_id = txn2.digest();

        let mut mpooldb = LeftRightMempool::new();

        match mpooldb.insert(txn1) {
            Ok(_) => {
                assert_eq!(1, mpooldb.size());
            },
            Err(_) => {
                panic!("Adding first transaction was unsuccesful !");
            },
        };

        match mpooldb.insert(txn2) {
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
        let recv1_keypair = KeyPair::random();
        let recv2_keypair = KeyPair::random();

        let sender_address = Address::new(keypair.get_miner_public_key().clone());
        let recv1_address = Address::new(recv1_keypair.get_miner_public_key().clone());
        let recv2_address = Address::new(recv2_keypair.get_miner_public_key().clone());

        let txn1 = TransactionKind::transfer_builder()
            .timestamp(0)
            .sender_address(sender_address.clone())
            .sender_public_key(keypair.get_miner_public_key().clone())
            .receiver_address(recv1_address.clone())
            .amount(0)
            .validators(HashMap::<String, bool>::new())
            .nonce(0)
            .signature(mock_txn_signature())
            .build_kind()
            .expect("Failed to build transaction");

        let txn2 = TransactionKind::transfer_builder()
            .timestamp(0)
            .sender_address(sender_address.clone())
            .sender_public_key(keypair.get_miner_public_key().clone())
            .receiver_address(recv2_address.clone())
            .amount(0)
            .validators(HashMap::<String, bool>::new())
            .nonce(0)
            .signature(mock_txn_signature())
            .build_kind()
            .expect("Failed to build transaction");

        let mut mpooldb = LeftRightMempool::new();

        match mpooldb.insert(txn1.clone()) {
            Ok(_) => {
                assert_eq!(1, mpooldb.size());
            },
            Err(_) => {
                panic!("Adding first transaction was unsuccesful !");
            },
        };

        match mpooldb.insert(txn2) {
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
        let recv_keypair = KeyPair::random();

        let mut txns = HashSet::<TransactionKind>::new();

        // let txn_id = String::from("1");
        let sender_address = Address::new(keypair.get_miner_public_key().clone());
        let recv_address = Address::new(recv_keypair.get_miner_public_key().clone());
        let txn_amount: u128 = 1010101;

        let transfer_builder = TransactionKind::transfer_builder()
            .timestamp(0)
            .sender_address(sender_address.clone())
            .sender_public_key(keypair.get_miner_public_key().clone())
            .receiver_address(recv_address.clone())
            .validators(HashMap::<String, bool>::new())
            .nonce(0)
            .signature(mock_txn_signature());

        for n in 1..101 {
            let txn = transfer_builder
                .clone()
                .amount(txn_amount + n)
                .build_kind()
                .expect("Failed to build transaction");

            txns.insert(txn);
        }

        let mut mpooldb = LeftRightMempool::new();
        match mpooldb.extend(txns.clone()) {
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
        let mut txns = HashSet::<TransactionKind>::new();

        let txn_amount: u128 = 1010101;

        let transfer_builder = TransactionKind::transfer_builder()
            .timestamp(0)
            .sender_address(Address::new(keypair.get_miner_public_key().clone()))
            .sender_public_key(keypair.get_miner_public_key().clone())
            .validators(HashMap::<String, bool>::new())
            .nonce(0)
            .signature(mock_txn_signature());

        for n in 1..u128::try_from(txn_id_max).unwrap_or(0) {
            let recv_keypair = KeyPair::random();
            let recv_address = Address::new(recv_keypair.get_miner_public_key().clone());

            let txn = transfer_builder
                .clone()
                .receiver_address(recv_address.clone())
                .amount(txn_amount + n)
                .build_kind()
                .expect("Failed to build transaction");

            txns.insert(txn);
        }

        match lrmpooldb.extend(txns) {
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
