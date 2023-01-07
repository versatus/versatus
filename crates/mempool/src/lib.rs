pub mod error;
pub mod mempool;

// TODO: remove deprecated modules after consolidating their internals into
// mempool
#[deprecated(note = "use mempool::Mempool instead")]
pub mod ev_mem_pool;
#[deprecated(note = "use mempool::Mempool instead")]
pub mod pool;

#[cfg(test)]
mod tests {

    use std::{
        collections::{HashMap, HashSet},
        time::{SystemTime, UNIX_EPOCH},
    };

    use rand::{thread_rng, Rng};
    use vrrb_core::{
        keypair::KeyPair,
        txn::{NewTxnArgs, Txn},
    };

    use crate::mempool::{LeftRightMemPoolDB, TxnStatus};

    #[test]
    fn creates_new_lrmempooldb() {
        let lrmpooldb = LeftRightMemPoolDB::new();
        assert_eq!(0, lrmpooldb.size().0);
    }

    #[test]
    fn add_a_single_txn() {
        let keypair = KeyPair::random();

        let txn = Txn::new(NewTxnArgs {
            sender_address: String::from("aaa1"),
            sender_public_key: keypair.get_miner_public_key().serialize().to_vec(),
            receiver_address: String::from("bbb1"),
            token: None,
            amount: 0,
            payload: Some(String::from("x")),
            validators: Some(HashMap::<String, bool>::new()),
            nonce: 0,
            signature: vec![],
        });

        let mut mpooldb = LeftRightMemPoolDB::new();
        match mpooldb.add_txn(&txn, TxnStatus::Pending) {
            Ok(_) => {
                std::thread::sleep(std::time::Duration::from_secs(3));
                assert_eq!(1, mpooldb.size().0);
            },
            Err(_) => {
                panic!("Adding first transaction was unsuccesful !");
            },
        };

        assert_eq!(1, mpooldb.size().0);
    }

    #[test]
    fn add_twice_same_txn() {
        let keypair = KeyPair::random();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let txn = Txn::new(NewTxnArgs {
            sender_address: String::from("aaa1"),
            sender_public_key: keypair.get_miner_public_key().serialize().to_vec(),
            receiver_address: String::from("bbb1"),
            token: None,
            amount: 0,
            payload: Some(String::from("x")),
            validators: Some(HashMap::<String, bool>::new()),
            nonce: 0,
            signature: vec![],
        });

        let mut mpooldb = LeftRightMemPoolDB::new();

        match mpooldb.add_txn(&txn, TxnStatus::Pending) {
            Ok(_) => {
                assert_eq!(1, mpooldb.size().0);
            },
            Err(_) => {
                panic!("Adding first transaction was unsuccesful !");
            },
        };

        match mpooldb.add_txn(&txn, TxnStatus::Pending) {
            Ok(_) => {
                assert_eq!(1, mpooldb.size().0);
            },
            Err(_) => {
                assert_eq!(1, mpooldb.size().0);
            },
        };

        assert_eq!(1, mpooldb.size().0);
    }

    #[test]
    fn add_two_different_txn() {
        let keypair = KeyPair::random();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let txn1 = Txn::new(NewTxnArgs {
            sender_address: String::from("aaa1"),
            sender_public_key: keypair.get_miner_public_key().serialize().to_vec(),
            receiver_address: String::from("bbb1"),
            token: None,
            amount: 0,
            payload: Some(String::from("x")),
            validators: Some(HashMap::<String, bool>::new()),
            nonce: 0,
            signature: vec![],
        });

        let txn2 = Txn::new(NewTxnArgs {
            sender_address: String::from("aaa1"),
            sender_public_key: keypair.get_miner_public_key().serialize().to_vec(),
            receiver_address: String::from("ccc1"),
            token: None,
            amount: 0,
            payload: Some(String::from("x")),
            validators: Some(HashMap::<String, bool>::new()),
            nonce: 0,
            signature: vec![],
        });

        let mut mpooldb = LeftRightMemPoolDB::new();

        match mpooldb.add_txn(&txn1, TxnStatus::Pending) {
            Ok(_) => {
                assert_eq!(1, mpooldb.size().0);
            },
            Err(_) => {
                panic!("Adding first transaction was unsuccesful !");
            },
        };

        match mpooldb.add_txn(&txn2, TxnStatus::Pending) {
            Ok(_) => {
                assert_eq!(2, mpooldb.size().0);
            },
            Err(_) => {
                panic!("Adding another, different transaction was unsuccesful !");
            },
        };
    }

    #[test]
    fn add_and_retrieve_txn() {
        let keypair = KeyPair::random();
        let sender_address = String::from("aaa1");
        let receiver_address = String::from("bbb1");
        let txn_amount: u128 = 1010101;

        let txn = Txn::new(NewTxnArgs {
            sender_address: String::from("aaa1"),
            sender_public_key: keypair.get_miner_public_key().serialize().to_vec(),
            receiver_address: String::from("bbb1"),
            token: None,
            amount: 0,
            payload: Some(String::from("x")),
            validators: Some(HashMap::<String, bool>::new()),
            nonce: 0,
            signature: vec![],
        });

        let txn_id = txn.txn_id();

        let mut mpooldb = LeftRightMemPoolDB::new();
        match mpooldb.add_txn(&txn, TxnStatus::Pending) {
            Ok(_) => {
                assert_eq!(1, mpooldb.size().0);
            },
            Err(_) => {
                panic!("Adding transaction was unsuccesful !");
            },
        };

        let now = chrono::offset::Utc::now().timestamp();

        // Test single Txn retrieval
        if let Some(txn_retrieved) = mpooldb.get_txn(&txn.txn_id().clone()) {
            assert_eq!(txn_retrieved.txn_id(), txn_id);
            assert_eq!(txn_retrieved.timestamp, now);
            assert_eq!(txn_retrieved.sender_address, sender_address);
            assert_eq!(txn_retrieved.receiver_address, receiver_address);
            assert_eq!(txn_retrieved.amount(), txn_amount);
        } else {
            panic!("No transaction found!");
        }

        // Test TxnRecord retrieval
        if let Some(txn_rec_retrieved) = mpooldb.get_txn_record(&txn.txn_id().clone()) {
            let txn_retrieved = Txn::from_string(&txn_rec_retrieved.txn);
            assert_eq!(txn_retrieved.txn_id(), txn_id);
            assert_eq!(txn_retrieved.timestamp, now);
            assert_eq!(txn_retrieved.sender_address, sender_address);
            assert_eq!(txn_retrieved.receiver_address, receiver_address);
            assert_eq!(txn_retrieved.amount(), txn_amount);
        } else {
            panic!("No transaction found!");
        }
    }

    #[test]
    fn add_batch_of_transactions() {
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
                sender_address: String::from("aaa1"),
                sender_public_key: keypair.get_miner_public_key().serialize().to_vec(),
                receiver_address: receiver_address.clone(),
                token: None,
                amount: txn_amount + n,
                payload: Some(String::from("x")),
                validators: Some(HashMap::<String, bool>::new()),
                nonce: 0,
                signature: vec![],
            });

            let txn_ser = txn.to_string();

            txns.insert(Txn::from_string(&txn_ser));
        }

        let mut mpooldb = LeftRightMemPoolDB::new();
        match mpooldb.add_txn_batch(&txns, TxnStatus::Pending) {
            Ok(_) => {
                assert_eq!(100, mpooldb.size().0);
            },
            Err(_) => {
                panic!("Adding transaction was unsuccesful !");
            },
        };

        let index = thread_rng().gen_range(0..txns.len());
        let test_txn_id = txns.iter().nth(index).unwrap().clone().txn_id();
        let test_txn_amount = txns.iter().nth(index).unwrap().clone().amount();

        if let Some(txn_retrieved) = mpooldb.get_txn(&test_txn_id) {
            assert_eq!(txn_retrieved.sender_address, sender_address);
            assert_eq!(txn_retrieved.receiver_address, receiver_address);
            assert_eq!(txn_retrieved.amount(), test_txn_amount);
        } else {
            panic!("No transaction found!");
        }
    }

    #[test]
    fn remove_single_txn_by_id() {
        let keypair = KeyPair::random();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let txn1 = Txn::new(NewTxnArgs {
            sender_address: String::from("aaa1"),
            sender_public_key: keypair.get_miner_public_key().serialize().to_vec(),
            receiver_address: String::from("bbb1"),
            token: None,
            amount: 0,
            payload: Some(String::from("x")),
            validators: Some(HashMap::<String, bool>::new()),
            nonce: 0,
            signature: vec![],
        });

        let txn2 = Txn::new(NewTxnArgs {
            sender_address: String::from("aaa1"),
            sender_public_key: keypair.get_miner_public_key().serialize().to_vec(),
            receiver_address: String::from("ccc1"),
            token: None,
            amount: 0,
            payload: Some(String::from("x")),
            validators: Some(HashMap::<String, bool>::new()),
            nonce: 0,
            signature: vec![],
        });

        let txn2_id = txn2.txn_id();

        let mut mpooldb = LeftRightMemPoolDB::new();

        match mpooldb.add_txn(&txn1, TxnStatus::Pending) {
            Ok(_) => {
                assert_eq!(1, mpooldb.size().0);
            },
            Err(_) => {
                panic!("Adding first transaction was unsuccesful !");
            },
        };

        match mpooldb.add_txn(&txn2, TxnStatus::Pending) {
            Ok(_) => {
                assert_eq!(2, mpooldb.size().0);
            },
            Err(_) => {
                panic!("Adding another, different transaction was unsuccesful !");
            },
        };

        match mpooldb.remove_txn_by_id(txn2_id.clone()) {
            Ok(_) => {
                assert_eq!(1, mpooldb.size().0);
            },
            Err(_) => {
                panic!("Adding another, different transaction was unsuccesful !");
            },
        };
    }

    #[test]
    fn remove_single_txn() {
        let keypair = KeyPair::random();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let txn1 = Txn::new(NewTxnArgs {
            sender_address: String::from("aaa1"),
            sender_public_key: keypair.get_miner_public_key().serialize().to_vec(),
            receiver_address: String::from("bbb1"),
            token: None,
            amount: 0,
            payload: Some(String::from("x")),
            validators: Some(HashMap::<String, bool>::new()),
            nonce: 0,
            signature: vec![],
        });

        let txn2 = Txn::new(NewTxnArgs {
            sender_address: String::from("aaa1"),
            sender_public_key: keypair.get_miner_public_key().serialize().to_vec(),
            receiver_address: String::from("ccc1"),
            token: None,
            amount: 0,
            payload: Some(String::from("x")),
            validators: Some(HashMap::<String, bool>::new()),
            nonce: 0,
            signature: vec![],
        });

        let mut mpooldb = LeftRightMemPoolDB::new();

        match mpooldb.add_txn(&txn1, TxnStatus::Pending) {
            Ok(_) => {
                assert_eq!(1, mpooldb.size().0);
            },
            Err(_) => {
                panic!("Adding first transaction was unsuccesful !");
            },
        };

        match mpooldb.add_txn(&txn2, TxnStatus::Pending) {
            Ok(_) => {
                assert_eq!(2, mpooldb.size().0);
            },
            Err(_) => {
                panic!("Adding another, different transaction was unsuccesful !");
            },
        };

        match mpooldb.remove_txn(&txn1, TxnStatus::Pending) {
            Ok(_) => {
                assert_eq!(1, mpooldb.size().0);
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
                sender_address: String::from("aaa1"),
                sender_public_key: keypair.get_miner_public_key().serialize().to_vec(),
                receiver_address: receiver_address.clone(),
                token: None,
                amount: txn_amount + n,
                payload: Some(String::from("x")),
                validators: Some(HashMap::<String, bool>::new()),
                nonce: 0,
                signature: vec![],
            });

            let txn_ser = txn.to_string();

            txns.insert(Txn::from_string(&txn_ser));
        }

        let mut mpooldb = LeftRightMemPoolDB::new();
        match mpooldb.add_txn_batch(&txns, TxnStatus::Pending) {
            Ok(_) => {
                assert_eq!(100, mpooldb.size().0);
            },
            Err(_) => {
                panic!("Adding transactions was unsuccesful !");
            },
        };
        match mpooldb.remove_txn_batch(&txns, TxnStatus::Pending) {
            Ok(_) => {
                assert_eq!(0, mpooldb.size().0);
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
        let mut lrmpooldb = LeftRightMemPoolDB::new();
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
                sender_address: String::from("aaa1"),
                sender_public_key: keypair.get_miner_public_key().serialize().to_vec(),
                receiver_address: receiver_address.clone(),
                token: None,
                amount: txn_amount + n,
                payload: Some(String::from("x")),
                validators: Some(HashMap::<String, bool>::new()),
                nonce: 0,
                signature: vec![],
            });

            txns.insert(txn);
        }

        match lrmpooldb.add_txn_batch(&txns, TxnStatus::Pending) {
            Ok(_) => {
                assert_eq!(txn_id_max - 1, lrmpooldb.size().0);
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

                    match read_hdl.enter().map(|guard| guard.clone()) {
                        Some(m) => {
                            assert_eq!(m.pending.len(), txn_id_max - 1);
                        },
                        None => {
                            panic!("No mempool !");
                        },
                    };
                })
            })
            .for_each(|handle| {
                handle.join().unwrap();
            });
    }
}
