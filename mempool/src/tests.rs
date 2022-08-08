
#[cfg(test)]
mod tests {

    use txn::txn::Txn;

    use crate::mempool::Mempool;
    use crate::mempool::MempoolDB;
    use std::{
        collections::HashMap,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn creates_new_mempool() {
        let mpooldb = Mempool::new();
        assert_eq!(0, mpooldb.size());
    }

    #[test]
    fn add_a_single_txn() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let txn = Txn {
            txn_id: String::from("1"),
            txn_timestamp: now,
            sender_address: String::from("aaa1"),
            sender_public_key: String::from("RSA"),
            receiver_address: String::from("bbb1"),
            txn_token: None,
            txn_amount: 0,
            txn_payload: String::from("x"),
            txn_signature: String::from("x"),
            validators: HashMap::<String, bool>::new(),
            nonce: 0,
        };

        let mut mpooldb = MempoolDB::new();
        match mpooldb.add_txn(&txn) {
            Ok(_) => {
                assert_eq!(1, mpooldb.size());
            },
            Err(_) => {
                panic!("Adding first transaction was unsuccesful !");
            }
        };

        assert_eq!(1, mpooldb.size());
    }

    #[test]
    fn add_twice_same_txn() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let txn = Txn {
            txn_id: String::from("1"),
            txn_timestamp: now,
            sender_address: String::from("aaa1"),
            sender_public_key: String::from("RSA"),
            receiver_address: String::from("bbb1"),
            txn_token: None,
            txn_amount: 0,
            txn_payload: String::from("x"),
            txn_signature: String::from("x"),
            validators: HashMap::<String, bool>::new(),
            nonce: 0,
        };

        let mut mpooldb = MempoolDB::new();

        match mpooldb.add_txn(&txn) {
            Ok(_) => {
                assert_eq!(1, mpooldb.size());
            },
            Err(_) => {
                panic!("Adding first transaction was unsuccesful !");
            }
        };

        match mpooldb.add_txn(&txn) {
            Ok(_) => {
                panic!("Adding second identical transaction was succesful !");
            },
            Err(_) => {
                assert_eq!(1, mpooldb.size());
            }
        };

        assert_eq!(1, mpooldb.size());
    }

    #[test]
    fn add_two_different_txn() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let txn1 = Txn {
            txn_id: String::from("1"),
            txn_timestamp: now,
            sender_address: String::from("aaa1"),
            sender_public_key: String::from("RSA"),
            receiver_address: String::from("bbb1"),
            txn_token: None,
            txn_amount: 0,
            txn_payload: String::from("x"),
            txn_signature: String::from("x"),
            validators: HashMap::<String, bool>::new(),
            nonce: 0,
        };

        let txn2 = Txn {
            txn_id: String::from("2"),
            txn_timestamp: now,
            sender_address: String::from("aaa1"),
            sender_public_key: String::from("RSA"),
            receiver_address: String::from("ccc1"),
            txn_token: None,
            txn_amount: 0,
            txn_payload: String::from("x"),
            txn_signature: String::from("x"),
            validators: HashMap::<String, bool>::new(),
            nonce: 0,
        };

        let mut mpooldb = MempoolDB::new();

        match mpooldb.add_txn(&txn1) {
            Ok(_) => {
                assert_eq!(1, mpooldb.size());
            },
            Err(_) => {
                panic!("Adding first transaction was unsuccesful !");
            }
        };

        match mpooldb.add_txn(&txn2) {
            Ok(_) => {
                assert_eq!(2, mpooldb.size());
            },
            Err(_) => {
                panic!("Adding another, different transaction was unsuccesful !");
            }
        };

    }

    #[test]
    fn add_and_retrieve_txn() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let txn_id = String::from("1");
        let sender_address = String::from("aaa1");
        let receiver_address = String::from("bbb1");
        let txn_amount: u128 = 1010101;

        let txn = Txn {
            txn_id: txn_id.clone(),
            txn_timestamp: now,
            sender_address: sender_address.clone(),
            sender_public_key: String::from("RSA"),
            receiver_address: receiver_address.clone(),
            txn_token: None,
            txn_amount: txn_amount,
            txn_payload: String::from("x"),
            txn_signature: String::from("x"),
            validators: HashMap::<String, bool>::new(),
            nonce: 0,
        };

        let mut mpooldb = MempoolDB::new();
        match mpooldb.add_txn(&txn) {
            Ok(_) => {
                assert_eq!(1, mpooldb.size());
            },
            Err(_) => {
                panic!("Adding transaction was unsuccesful !");
            }
        };

        if let Some(txn_retrieved) = mpooldb.get_txn(txn.txn_id.clone()) {
            assert_eq!(txn_retrieved.txn_id, txn_id);
            assert_eq!(txn_retrieved.txn_timestamp, now);
            assert_eq!(txn_retrieved.sender_address, sender_address);
            assert_eq!(txn_retrieved.receiver_address, receiver_address);
            assert_eq!(txn_retrieved.txn_amount, txn_amount);
        } else {
            panic!("No transaction found!");
        }
    }

}
