use std::collections::HashMap;

use primitives::Address;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use vrrb_core::{
    account::{Account, AccountDigests},
    keypair::Keypair,
};
use vrrbdb::{VrrbDb, VrrbDbConfig};

mod common;
use common::_generate_random_address;
use serial_test::serial;

#[test]
#[serial]
fn accounts_can_be_added() {
    let mut db = VrrbDb::new(VrrbDbConfig::default());

    let (_, addr1) = _generate_random_address();
    let (_, addr2) = _generate_random_address();
    let (_, addr3) = _generate_random_address();
    let (_, addr4) = _generate_random_address();
    let (_, addr5) = _generate_random_address();

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
            digests: AccountDigests::default(),
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
            digests: AccountDigests::default(),
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
                digests: AccountDigests::default(),
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
                digests: AccountDigests::default(),
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
                digests: AccountDigests::default(),
                created_at: 0,
                updated_at: None,
            },
        ),
    ]);

    let entries = db.state_store_factory().handle().entries();

    assert_eq!(entries.len(), 5);
}
