use std::{collections::HashMap, env, fs};

use primitives::Address;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use vrrb_core::{
    account::Account,
    keypair::Keypair,
    txn::{NewTxnArgs, Txn},
};
use vrrbdb::{VrrbDb, VrrbDbConfig};
use serial_test::serial;
mod common;

use common::{
    generate_random_address,
    generate_random_string,
    generate_random_transaction,
    generate_random_valid_transaction,
};

#[test]
#[serial]
fn transactions_can_be_added() {
    let temp_dir_path = env::temp_dir();
    let state_backup_path = temp_dir_path.join(format!("{}", generate_random_string()));

    let mut db = VrrbDb::new(VrrbDbConfig {
        path: state_backup_path,
        state_store_path: None,
        transaction_store_path: None,
        event_store_path: None,
    });

    let txn1 = generate_random_valid_transaction();
    let txn2 = generate_random_valid_transaction();

    db.insert_transaction_unchecked(txn1).unwrap();
    db.insert_transaction_unchecked(txn2).unwrap();

    let entries = db.transaction_store_factory().handle().entries();

    assert_eq!(entries.len(), 2);

    db.extend_transactions_unchecked(vec![
        generate_random_valid_transaction(),
        generate_random_valid_transaction(),
        generate_random_valid_transaction(),
    ]);

    let entries = db.transaction_store_factory().handle().entries();

    assert_eq!(entries.len(), 5);
}
