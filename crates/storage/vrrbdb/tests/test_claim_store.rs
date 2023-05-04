use vrrbdb::{VrrbDb, VrrbDbConfig};

mod common;
use common::generate_random_claim;
use serial_test::serial;


#[test]
#[serial]
fn claims_can_be_added() {
    let mut db = VrrbDb::new(VrrbDbConfig::default());

    let claim1 = generate_random_claim();
    let claim2 = generate_random_claim();
    let claim3 = generate_random_claim();
    let claim4 = generate_random_claim();
    let claim5 = generate_random_claim();

    db.insert_claim(claim1).unwrap();

    db.insert_claim(claim2).unwrap();

    let entries = db.claim_store_factory().handle().entries();

    assert_eq!(entries.len(), 2);

    db.extend_claims(vec![
        (claim3.hash, claim3),
        (claim4.hash, claim4),
        (claim5.hash, claim5),
    ]);

    let entries = db.claim_store_factory().handle().entries();

    assert_eq!(entries.len(), 5);
}
