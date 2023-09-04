use std::env;

use patriecia::{KeyHash, Sha256};
use serial_test::serial;
use vrrbdb::{VrrbDb, VrrbDbConfig};
mod common;

use common::{_generate_random_string, _generate_random_valid_transaction};

#[test]
#[serial]
fn transactions_can_be_added() {
    let temp_dir_path = env::temp_dir();
    let state_backup_path = temp_dir_path.join(format!("{}", _generate_random_string()));

    let mut db = VrrbDb::new(VrrbDbConfig {
        path: state_backup_path,
        state_store_path: None,
        transaction_store_path: None,
        event_store_path: None,
        claim_store_path: None,
    });

    let txn1 = _generate_random_valid_transaction();
    let txn2 = _generate_random_valid_transaction();

    db.insert_transaction_unchecked(txn1).unwrap();
    db.insert_transaction_unchecked(txn2).unwrap();

    let entries = db.transaction_store_factory().handle().entries();

    assert_eq!(entries.len(), 2);

    db.extend_transactions_unchecked(vec![
        _generate_random_valid_transaction(),
        _generate_random_valid_transaction(),
        _generate_random_valid_transaction(),
    ]);

    let entries = db.transaction_store_factory().handle().entries();

    assert_eq!(entries.len(), 5);
}
