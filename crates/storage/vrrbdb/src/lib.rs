mod claim_store;
pub mod result;
mod rocksdb_adapter;
mod state_store;
pub mod test_utils;
mod transaction_store;
pub mod types;
mod vrrbdb;
mod vrrbdb_read_handle;
mod vrrbdb_serialized_values;

pub use claim_store::*;
pub use rocksdb_adapter::*;
pub use state_store::*;
pub use transaction_store::*;
pub use types::*;
pub use vrrbdb_read_handle::*;
pub use vrrbdb_serialized_values::*;

pub use crate::vrrbdb::*;

#[cfg(test)]
mod tests {

    // fn new_random_keys(n: usize) -> Vec<PublicKeys> {
    //     let mut res: Vec<PublicKeys> = vec![];
    //     for _ in 0..n {
    //         let keypair = KeyPair::random();
    //         res.push((
    //             keypair.get_miner_public_key().clone(),
    //             keypair.get_validator_public_key().clone(),
    //         ));
    //     }
    //     res
    // }
    //
    // #[test]
    // fn creates_new_database() {
    //     let vdb = VrrbDb::new();
    //     assert_eq!(vdb.len(), 0);
    // }
    //
    // #[test]
    // fn commiting_change_changes_last_refresh() {
    //     let mut vdb = VrrbDb::new();
    //     let key = new_random_keys(1);
    //     let initial_time = vdb.last_refresh();
    //     thread::sleep(Duration::from_secs(1));
    //     _ = vdb.insert(key[0], Account::new());
    //     assert_ne!(initial_time, vdb.last_refresh());
    // }
    //
    // #[test]
    // fn insert_single_account_with_commitment() {
    //     let keys = new_random_keys(10);
    //
    //     let mut account = Account::new();
    //     _ = account.update_field(AccountField::Credits(100));
    //
    //     let mut vdb = VrrbDb::new();
    //     if let Err(e) = vdb.insert(keys[0], account) {
    //         panic!("Failed to insert account with commitment: {:?}", e)
    //     }
    //
    //     if let Some(account) = vdb.read_handle().get(keys[0]) {
    //         assert_eq!(account.credits, 100);
    //     } else {
    //         panic!("Failed to get the account")
    //     }
    // }
    //
    // #[test]
    // fn fail_to_insert_with_nonce() {
    //     let keys = new_random_keys(10);
    //     let mut vdb = VrrbDb::new();
    //     let mut record = Account::new();
    //
    //     record.bump_nonce();
    //
    //     let result = vdb.insert(keys[0], record);
    //     assert_eq!(result, Err(VrrbDbError::InitWithNonce))
    // }
    //
    // #[test]
    // fn fail_to_insert_with_debit() {
    //     let keys = new_random_keys(10);
    //     let mut vdb = VrrbDb::new();
    //     let mut account = Account::new();
    //     _ = account.update_field(AccountField::Credits(200));
    //     _ = account.update_field(AccountField::Debits(100));
    //     account.bump_nonce();
    //
    //     let result = vdb.insert(keys[0], account);
    //     assert_eq!(result, Err(VrrbDbError::InitWithDebit))
    // }
    //
    // #[test]
    // fn inserts_multiple_valid_k_v_pairs_with_commitment() {
    //     let keys = new_random_keys(10);
    //     let mut vdb = VrrbDb::new();
    //
    //     let mut record1 = Account::new();
    //     _ = record1.update_field(AccountField::Credits(100));
    //     _ = record1.update_field(AccountField::Debits(0));
    //
    //     let mut record2 = Account::new();
    //     _ = record2.update_field(AccountField::Credits(237));
    //     _ = record2.update_field(AccountField::Debits(0));
    //
    //     let mut record3 = Account::new();
    //     _ = record3.update_field(AccountField::Credits(500));
    //     _ = record3.update_field(AccountField::Debits(0));
    //
    //     vdb.batch_insert(vec![
    //         (keys[0], record1),
    //         (keys[1], record2),
    //         (keys[2], record3),
    //     ]);
    //
    //     if let Some(account) = vdb.read_handle().get(keys[1]) {
    //         assert_eq!(account.credits, 237);
    //     }
    // }
    //
    // #[test]
    // fn insert_for_same_key_multiple_times_should_be_impossible() {
    //     let keys = new_random_keys(10);
    //     let mut vdb = VrrbDb::new();
    //
    //     let mut record1 = Account::new();
    //     _ = record1.update_field(AccountField::Credits(100));
    //     _ = record1.update_field(AccountField::Debits(0));
    //
    //     let mut record2 = Account::new();
    //     _ = record2.update_field(AccountField::Credits(237));
    //     _ = record2.update_field(AccountField::Debits(0));
    //
    //     if let Err(e) = vdb.insert(keys[0], record1) {
    //         panic!("Failed to update the account: {:?}", e);
    //     }
    //
    //     match vdb.insert(keys[0], record2) {
    //         Err(e) => {
    //             assert_eq!(e, VrrbDbError::RecordExists)
    //         },
    //         Ok(_) => {
    //             panic!("Multiple inserts for the same key!");
    //         },
    //     }
    // }
    //
    // #[test]
    // fn inserts_multiple_k_v_pairs_some_invalid_with_commitment() {
    //     let keys = new_random_keys(10);
    //     let mut vdb = VrrbDb::new();
    //
    //     let mut record1 = Account::new();
    //     _ = record1.update_field(AccountField::Credits(100));
    //     _ = record1.update_field(AccountField::Debits(0));
    //
    //     let mut record2 = Account::new();
    //     _ = record2.update_field(AccountField::Credits(237));
    //     _ = record2.update_field(AccountField::Debits(0));
    //
    //     let mut record3 = Account::new();
    //     _ = record3.update_field(AccountField::Credits(500));
    //     _ = record3.update_field(AccountField::Debits(500));
    //
    //     match vdb.batch_insert(vec![
    //         (keys[0], record1.clone()),
    //         (keys[0], record2.clone()),
    //         (keys[2], record3.clone()),
    //     ]) {
    //         None => {
    //             panic!("Should fail.")
    //         },
    //         Some(fails) => {
    //             let expected = vec![
    //                 (keys[0], record2, VrrbDbError::RecordExists),
    //                 (keys[2], record3, VrrbDbError::InitWithDebit),
    //             ];
    //             for i in 0..2 {
    //                 assert_eq!(expected[i].0, fails[i].0);
    //                 assert_eq!(expected[i].1, fails[i].1);
    //                 assert_eq!(expected[i].2, fails[i].2);
    //             }
    //         },
    //     }
    // }
    //
    // #[test]
    // fn retain_properly_filters_the_values() {
    //     let keys = new_random_keys(10);
    //     let mut vdb = VrrbDb::new();
    //
    //     let mut record = Account::new();
    //     _ = record.update_field(AccountField::Credits(123));
    //
    //     let mut record1 = Account::new();
    //     _ = record1.update_field(AccountField::Credits(250));
    //
    //     let mut record2 = Account::new();
    //     _ = record2.update_field(AccountField::Credits(300));
    //
    //     let mut record3 = Account::new();
    //     _ = record3.update_field(AccountField::Credits(500));
    //
    //     vdb.batch_insert(vec![
    //         (keys[0], record),
    //         (keys[1], record1),
    //         (keys[2], record2.clone()),
    //         (keys[3], record3),
    //     ]);
    //
    //     let filtered = vdb.retain(|acc| acc.credits >= 300 && acc.credits <
    // 500);     // filtered.r.handle().for_each(|key, value| {
    //     //     let account = *value[0].clone();
    //     //     assert_eq!(*key, keys[2]);
    //     //     assert_eq!(account, record2);
    //     // });
    //     assert_eq!(filtered.len(), 1);
    //     assert_ne!(filtered.read_handle().get(keys[2]), None);
    // }
    //
    // #[test]
    // fn retain_with_some_more_filters() {
    //     let keys = new_random_keys(10);
    //     let mut vdb = VrrbDb::new();
    //
    //     let account = Account::new();
    //     let updates = vec![
    //         AccountFieldsUpdate {
    //             nonce: account.nonce + 1,
    //             credits: Some(1230),
    //             debits: Some(10),
    //             storage: Some(Some("Some storage".to_string())),
    //             ..Default::default()
    //         },
    //         AccountFieldsUpdate {
    //             credits: Some(100),
    //             debits: Some(300),
    //             nonce: account.nonce + 2,
    //             ..Default::default()
    //         },
    //     ];
    //
    //     let account1 = Account::new();
    //     let update1 = AccountFieldsUpdate {
    //         nonce: account1.nonce + 1,
    //         credits: Some(250),
    //         ..Default::default()
    //     };
    //
    //     let account2 = Account::new();
    //     let update2 = AccountFieldsUpdate {
    //         nonce: account2.nonce + 1,
    //         credits: Some(300),
    //         debits: Some(250),
    //         code: Some(Some("test".to_string())),
    //         ..Default::default()
    //     };
    //
    //     let account3 = Account::new();
    //     let updates3 = vec![
    //         AccountFieldsUpdate {
    //             nonce: account3.nonce + 1,
    //             credits: Some(500),
    //             debits: Some(500),
    //             ..Default::default()
    //         },
    //         AccountFieldsUpdate {
    //             nonce: account3.nonce + 2,
    //             credits: Some(90),
    //             ..Default::default()
    //         },
    //     ];
    //
    //     if let Some(failed) = vdb.batch_insert(vec![
    //         (keys[0], account.clone()),
    //         (keys[1], account1.clone()),
    //         (keys[2], account2.clone()),
    //         (keys[3], account3.clone()),
    //     ]) {
    //         failed.iter().for_each(|(_, _, e)| {
    //             println!("{:?}", e);
    //         });
    //     };
    //
    //     if let Some(fails) = vdb.batch_update(vec![
    //         (keys[0], updates[0].clone()),
    //         (keys[0], updates[1].clone()),
    //         (keys[1], update1),
    //         (keys[2], update2),
    //         (keys[3], updates3[0].clone()),
    //         (keys[3], updates3[1].clone()),
    //     ]) {
    //         panic!("Some updates failed {:?}", fails);
    //     };
    //
    //     // Only the first account passes that
    //     let filtered = vdb.retain(|acc| {
    //         (acc.hash.starts_with("a13c") || acc.hash.starts_with("a036"))
    //             && acc.credits - acc.debits > 50
    //             && (acc.storage.is_some() || acc.code.is_some())
    //     });
    //
    //     println!("{:?}", filtered.read_handle().get(keys[0]).unwrap().hash);
    //
    //     assert_eq!(filtered.len(), 1);
    // }
    //
    // #[test]
    // fn get_should_return_account() {
    //     let mut vdb = VrrbDb::new();
    //     let account = Account::new();
    //     let keys = new_random_keys(10);
    //
    //     _ = vdb.insert(keys[0], account.clone());
    //     assert_eq!(vdb.read_handle().get(keys[0]), Some(account));
    // }
    //
    // #[test]
    // fn get_should_return_none_for_nonexistant_account() {
    //     let vdb = VrrbDb::new();
    //     let keys = new_random_keys(10);
    //     assert_eq!(vdb.read_handle().get(keys[0]), None);
    // }
    //
    // #[test]
    // fn update_with_valid_fields_should_work() {
    //     let keys = new_random_keys(10);
    //     let mut vdb = VrrbDb::new();
    //
    //     let account = Account::new();
    //     let updates = vec![
    //         AccountFieldsUpdate {
    //             nonce: account.nonce + 1,
    //             credits: Some(1230),
    //             debits: Some(10),
    //             storage: Some(Some("Some storage".to_string())),
    //             ..Default::default()
    //         },
    //         AccountFieldsUpdate {
    //             credits: Some(100),
    //             debits: Some(300),
    //             nonce: account.nonce + 2,
    //             ..Default::default()
    //         },
    //     ];
    //
    //     let account1 = Account::new();
    //     let update1 = AccountFieldsUpdate {
    //         nonce: account1.nonce + 1,
    //         credits: Some(250),
    //         ..Default::default()
    //     };
    //
    //     let account2 = Account::new();
    //     let update2 = AccountFieldsUpdate {
    //         nonce: account2.nonce + 1,
    //         credits: Some(300),
    //         debits: Some(250),
    //         code: Some(Some("test".to_string())),
    //         ..Default::default()
    //     };
    //
    //     let account3 = Account::new();
    //     let updates3 = vec![
    //         AccountFieldsUpdate {
    //             nonce: account3.nonce + 1,
    //             credits: Some(500),
    //             debits: Some(500),
    //             ..Default::default()
    //         },
    //         AccountFieldsUpdate {
    //             nonce: account3.nonce + 2,
    //             credits: Some(90),
    //             ..Default::default()
    //         },
    //     ];
    //
    //     if let Some(failed) = vdb.batch_insert(vec![
    //         (keys[0], account.clone()),
    //         (keys[1], account1.clone()),
    //         (keys[2], account2.clone()),
    //         (keys[3], account3.clone()),
    //     ]) {
    //         failed.iter().for_each(|(_, _, e)| {
    //             println!("{:?}", e);
    //         });
    //     };
    //
    //     if let Some(fails) = vdb.batch_update(vec![
    //         (keys[0], updates[0].clone()),
    //         (keys[0], updates[1].clone()),
    //         (keys[1], update1),
    //         (keys[2], update2),
    //         (keys[3], updates3[0].clone()),
    //         (keys[3], updates3[1].clone()),
    //     ]) {
    //         panic!("Some updates failed {:?}", fails);
    //     };
    // }
    //
    // #[test]
    // fn update_batch_invalid_data_should_return_error() {
    //     let keys = new_random_keys(10);
    //     let mut vdb = VrrbDb::new();
    //
    //     let account = Account::new();
    //     let updates = vec![
    //         AccountFieldsUpdate {
    //             nonce: account.nonce + 1,
    //             credits: Some(1230),
    //             debits: Some(10),
    //             storage: Some(Some("Some storage".to_string())),
    //             ..Default::default()
    //         },
    //         // Invalid nonce
    //         AccountFieldsUpdate {
    //             credits: Some(100),
    //             debits: Some(300),
    //             nonce: account.nonce + 3,
    //             ..Default::default()
    //         },
    //     ];
    //
    //     let account1 = Account::new();
    //     let update1 = AccountFieldsUpdate {
    //         nonce: account1.nonce + 1,
    //         credits: Some(250),
    //         ..Default::default()
    //     };
    //
    //     let account2 = Account::new();
    //     let update2 = AccountFieldsUpdate {
    //         nonce: account2.nonce + 1,
    //         credits: Some(300),
    //         debits: Some(250),
    //         code: Some(Some("test".to_string())),
    //         ..Default::default()
    //     };
    //
    //     let account3 = Account::new();
    //     let updates3 = vec![
    //         AccountFieldsUpdate {
    //             nonce: account3.nonce + 1,
    //             credits: Some(500),
    //             debits: Some(500),
    //             ..Default::default()
    //         },
    //         // Invalid update - more debit than credit
    //         AccountFieldsUpdate {
    //             nonce: account3.nonce + 2,
    //             debits: Some(90),
    //             ..Default::default()
    //         },
    //     ];
    //
    //     if let Some(_) = vdb.batch_insert(vec![
    //         (keys[0], account.clone()),
    //         (keys[1], account1.clone()),
    //         (keys[2], account2.clone()),
    //         (keys[3], account3.clone()),
    //     ]) {
    //         panic!("Failed to insert accounts");
    //     };
    //
    //     if let Some(fails) = vdb.batch_update(vec![
    //         (keys[0], updates[0].clone()),
    //         (keys[0], updates[1].clone()),
    //         (keys[1], update1),
    //         (keys[2], update2),
    //         (keys[3], updates3[0].clone()),
    //         (keys[3], updates3[1].clone()),
    //     ]) {
    //         assert_eq!(fails.len(), 2);
    //
    //         let mut nonce_error_index = 1;
    //         let mut debits_error_index = 0;
    //         if fails[0].0 == keys[0] {
    //             nonce_error_index = 0;
    //             debits_error_index = 1;
    //         }
    //         assert_eq!(
    //             fails[debits_error_index].2,
    //             Err(VrrbDbError::UpdateFailed(AccountField::Debits(90)))
    //         );
    //         assert_eq!(
    //             fails[nonce_error_index].2,
    //             Err(VrrbDbError::InvalidUpdateNonce(
    //                 account.nonce + 1,
    //                 account.nonce + 3
    //             ))
    //         );
    //     };
    // }
    //
    // use std::time::Duration;
    // #[test]
    // fn concurrency_test_with_writes_and_reads() {
    //     const WRITES: usize = 50;
    //     let mut vdb = VrrbDb::new();
    //     let keys = new_random_keys(WRITES);
    //     let keys2 = keys.clone();
    //     let read_handle = vdb.read_handle();
    //     for _ in 0..10 {
    //         let rh = read_handle.clone();
    //         let keys = keys2.clone();
    //         thread::spawn(move || {
    //             for _ in 0..1000 {
    //                 rh.batch_get(keys[..].to_vec());
    //                 thread::sleep(Duration::from_millis(10));
    //                 print!(".")
    //             }
    //         });
    //     }
    //     thread::spawn(move || {
    //         for i in 0..WRITES {
    //             if let Err(e) = vdb.insert(keys[i], Account::new()) {
    //                 panic!("{:?}", e);
    //             };
    //             println!("{:?} {}", vdb.read_handle().get(keys[i]),
    // vdb.len());             print!(
    //                 "\nWrite at: {:?}\n Added account: {:?} \n Accounts in
    // db: {}",                 vdb.last_refresh(),
    //                 vdb.read_handle().get(keys[i]),
    //                 vdb.len()
    //             );
    //             thread::sleep(Duration::from_millis(100));
    //         }
    //     });
    //     thread::sleep(Duration::from_millis(1000));
    // }
}
