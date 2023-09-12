use std::collections::HashMap;

use primitives::NodeId;
use vrrb_core::claim::Claim;
use vrrbdb::{VrrbDb, VrrbDbConfig};

mod common;
use common::_generate_random_claim;
use serial_test::serial;

#[test]
#[serial]
fn claims_can_be_added() {
    let mut db = VrrbDb::new(VrrbDbConfig::default());

    let claim1 = _generate_random_claim();
    let claim2 = _generate_random_claim();
    let claim3 = _generate_random_claim();
    let claim4 = _generate_random_claim();
    let claim5 = _generate_random_claim();

    db.insert_claim(claim1).unwrap();

    db.insert_claim(claim2).unwrap();

    let entries: HashMap<NodeId, Claim> = db.claim_store_factory().handle().entries();

    assert_eq!(entries.len(), 2);

    db.extend_claims(vec![
        (claim3.hash, Some(claim3)),
        (claim4.hash, Some(claim4)),
        (claim5.hash, Some(claim5)),
    ]);

    let entries = db.claim_store_factory().handle().entries();

    assert_eq!(entries.len(), 5);
}
