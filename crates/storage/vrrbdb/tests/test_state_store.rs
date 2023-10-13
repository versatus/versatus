use vrrb_core::account::Account;
use vrrbdb::{VrrbDb, VrrbDbConfig};

mod common;
use common::_generate_random_address;
use serial_test::serial;

#[test]
#[serial]
fn accounts_can_be_added() {
    let mut db = VrrbDb::new(VrrbDbConfig::default());

    let (_secret_key, addr1) = _generate_random_address();
    let (_, addr2) = _generate_random_address();
    let (_, addr3) = _generate_random_address();
    let (_, addr4) = _generate_random_address();
    let (_, addr5) = _generate_random_address();

    db.insert_account(addr1.clone(), Account::new(addr1))
        .unwrap();
    db.insert_account(addr2.clone(), Account::new(addr2))
        .unwrap();
    let read_handle = db.state_store_factory().handle();

    let entries = read_handle.entries();

    assert_eq!(entries.len(), 2);

    db.extend_accounts(vec![
        (addr3.clone(), Some(Account::new(addr3))),
        (addr4.clone(), Some(Account::new(addr4))),
        (addr5.clone(), Some(Account::new(addr5))),
    ]);

    let entries = db.state_store_factory().handle().entries();

    assert_eq!(entries.len(), 5);
}
