use vrrb_core::account::{Account, AccountDigests};
use vrrbdb::{VrrbDb, VrrbDbConfig};

mod common;
use common::_generate_random_address;
use serial_test::serial;
use primitives::Address;

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
        Account::default()
    )
    .unwrap();

    db.insert_account(
        addr2,
        Account::default()
    )
    .unwrap();

    let entries = db.state_store_factory().handle().entries();

    assert_eq!(entries.len(), 2);

    db.extend_accounts(vec![
        (
            addr3,
            Account::default()
        ),
        (
            addr4,
            Account::default()
        ),
        (
            addr5,
            Account::default()
        ),
    ]);

    let entries = db.state_store_factory().handle().entries();

    assert_eq!(entries.len(), 5);
}
