use std::{collections::HashMap, env, fs};

use primitives::Address;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use vrrb_core::{account::Account, keypair::Keypair};
use vrrbdb::{VrrbDb, VrrbDbConfig};

mod common;
use common::generate_random_address;
use serial_test::serial;
use crate::common::generate_random_string;

#[test]
#[serial]
fn accounts_can_be_added() {
    let temp_dir_path = env::temp_dir();
    let state_backup_path = temp_dir_path.join(format!("{}", generate_random_string()));

    let mut db = VrrbDb::new(VrrbDbConfig {
        path: state_backup_path,
        state_store_path: None,
        transaction_store_path: None,
        event_store_path: None,
    });

    let (_, addr1) = generate_random_address();
    let (_, addr2) = generate_random_address();
    let (_, addr3) = generate_random_address();
    let (_, addr4) = generate_random_address();
    let (_, addr5) = generate_random_address();

    db.insert_account(
        addr1,
        Account {
            hash: String::from(""),
            nonce: 0,
            credits: 0,
            debits: 0,
            storage: None,
            code: None,
            pubkey: vec![],
            digests: HashMap::new(),
            created_at: 0,
            updated_at: None,
        },
    )
    .unwrap();

    db.insert_account(
        addr2,
        Account {
            hash: String::from(""),
            nonce: 0,
            credits: 0,
            debits: 0,
            storage: None,
            code: None,
            pubkey: vec![],
            digests: HashMap::new(),
            created_at: 0,
            updated_at: None,
        },
    )
    .unwrap();

    let entries = db.state_store_factory().handle().entries();

    assert_eq!(entries.len(), 2);

    db.extend_accounts(vec![
        (
            addr3,
            Account {
                hash: String::from(""),
                nonce: 0,
                credits: 0,
                debits: 0,
                storage: None,
                code: None,
                pubkey: vec![],
                digests: HashMap::new(),
                created_at: 0,
                updated_at: None,
            },
        ),
        (
            addr4,
            Account {
                hash: String::from(""),
                nonce: 0,
                credits: 0,
                debits: 0,
                storage: None,
                code: None,
                pubkey: vec![],
                digests: HashMap::new(),
                created_at: 0,
                updated_at: None,
            },
        ),
        (
            addr5,
            Account {
                hash: String::from(""),
                nonce: 0,
                credits: 0,
                debits: 0,
                storage: None,
                code: None,
                pubkey: vec![],
                digests: HashMap::new(),
                created_at: 0,
                updated_at: None,
            },
        ),
    ]);

    let entries = db.state_store_factory().handle().entries();

    assert_eq!(entries.len(), 5);
}
