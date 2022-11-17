pub mod error;
pub mod mempool;

#[cfg(test)]
mod tests {

    use std::collections::HashSet;

    use secp256k1::{PublicKey, Secp256k1, SecretKey};
    use txn::txn::{NativeToken, SystemInstruction, Transaction, TransferData};

    use crate::mempool::{LeftRightMemPoolDB, TxnStatus};

    #[test]
    fn creates_new_lrmempooldb() {
        let lrmpooldb = LeftRightMemPoolDB::new();
        assert_eq!(0, lrmpooldb.size().0);
    }

    #[test]
    fn add_a_single_txn() {
        let txn = Transaction::default();

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
        let txn = Transaction::default();

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
                // panic!("Adding second identical transaction was succesful
                //
                // !");
            },
            Err(_) => {
                assert_eq!(1, mpooldb.size().0);
            },
        };

        assert_eq!(1, mpooldb.size().0);
    }

    #[test]
    fn add_two_different_txn() {
        let txn1 = Transaction::default();

        let txn2 = Transaction::default();

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
        let txn = Transaction::default();

        let mut mpooldb = LeftRightMemPoolDB::new();
        match mpooldb.add_txn(&txn, TxnStatus::Pending) {
            Ok(_) => {
                assert_eq!(1, mpooldb.size().0);
            },
            Err(_) => {
                panic!("Adding transaction was unsuccesful !");
            },
        };

        // Test single Txn retrieval
        if let Some(txn_retrieved) = mpooldb.get_txn(&txn.get_id()) {
            assert_eq!(txn_retrieved, txn)
        } else {
            panic!("No transaction found!");
        }

        // Test TxnRecord retrieval
        if let Some(txn_rec_retrieved) = mpooldb.get_txn_record(&txn.get_id()) {
            let txn_retrieved = Transaction::from_string(&txn_rec_retrieved.txn);
            assert_eq!(txn_retrieved, txn);
        } else {
            panic!("No transaction found!");
        }
    }

    #[test]
    fn add_batch_of_transactions() {
        let mut txns = HashSet::<Transaction>::new();


        let secp = Secp256k1::new();
        let mut rng = rand::thread_rng();
        let secret = SecretKey::new(&mut rng);
        let pubkey = PublicKey::from_secret_key(&secp, &secret);

        let mut example_txn = Transaction::default();

        for n in 1..101 {
            let mut txn = Transaction::default();
            txn.instructions
                .push(SystemInstruction::Transfer(TransferData {
                    from: pubkey,
                    to: pubkey,
                    amount: NativeToken(n),
                }));
            if n == 51 {
                example_txn = txn.clone();
            }
            txns.insert(txn);
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


        if mpooldb.get_txn(&example_txn.get_id()).is_none() {
            panic!("No transaction found!");
        }
    }

    #[test]
    fn remove_single_txn_by_id() {
        let txn1 = Transaction::default();


        let txn2 = Transaction::default();

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
                panic!(
                    "Adding another, different transaction was unsuccesful
    !"
                );
            },
        };

        match mpooldb.remove_txn_by_id(&txn2.get_id(), TxnStatus::Pending) {
            Ok(_) => {
                assert_eq!(1, mpooldb.size().0);
            },
            Err(_) => {
                panic!(
                    "Adding another, different transaction was unsuccesful
    !"
                );
            },
        };
    }

    #[test]
    fn remove_single_txn() {
        let txn1 = Transaction::default();

        let txn2 = Transaction::default();

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
                panic!(
                    "Adding another, different transaction was unsuccesful
    !"
                );
            },
        };

        match mpooldb.remove_txn(&txn1, TxnStatus::Pending) {
            Ok(_) => {
                assert_eq!(1, mpooldb.size().0);
            },
            Err(_) => {
                panic!(
                    "Adding another, different transaction was unsuccesful
    !"
                );
            },
        };
    }

    #[test]
    fn remove_txn_batch() {
        let mut txns = HashSet::<Transaction>::new();


        for _ in 1..101 {
            let txn = Transaction::default();
            txns.insert(txn);
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
        let txn_id_max = 11;

        let mut lrmpooldb = LeftRightMemPoolDB::new();
        let mut txns = HashSet::<Transaction>::new();


        for _ in 1..u128::try_from(txn_id_max).unwrap_or(0) {
            let txn = Transaction::default();

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
