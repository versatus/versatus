pub type Nonce = u32;

// /// Stores information about given account.
// #[derive(Clone, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
// pub struct Account {
//     pub hash: String,
//     pub nonce: Nonce,
//     pub credits: u128,
//     pub debits: u128,
//     pub storage: Option<String>,
//     pub code: Option<String>,
// }
//
// impl Account {
//     /// Returns new, empty account.
//     ///
//     /// Examples:
//     /// ```
//     /// use lrdb::Account;
//     ///
//     /// let account = Account::new();
//     /// ```
//     pub fn new() -> Account {
//         let nonce = 0u32;
//         let credits = 0u128;
//         let debits = 0u128;
//         let storage = None;
//         let code = None;
//
//         let mut hasher = Sha256::new();
//         hasher.update(&nonce.to_be_bytes());
//         hasher.update(&credits.to_be_bytes());
//         hasher.update(&debits.to_be_bytes());
//         let hash = format!("{:x}", hasher.finalize());
//
//         Account {
//             hash,
//             nonce,
//             credits,
//             debits,
//             storage,
//             code,
//         }
//     }
//
//     /// Modifies accounts hash, recalculating it using account's fields.
//     fn update_hash(&mut self) {
//         let mut hasher = Sha256::new();
//         hasher.update(self.nonce.to_be_bytes());
//         hasher.update(self.credits.to_be_bytes());
//         hasher.update(self.debits.to_be_bytes());
//
//         if let Some(storage) = &self.storage {
//             hasher.update(storage.as_bytes());
//         }
//
//         if let Some(code) = &self.code {
//             hasher.update(code.as_bytes());
//         }
//         self.hash = format!("{:x}", hasher.finalize());
//     }
//
//     // TODO: do those safely
//     // Should we rollback credits and debits on overflow?
//     // e.g.
//     // self.credits -= self.debits;
//     // self.debits -= self.debits;
//     //
//     // This way overall balance stays the same
//     // But the numbers are fine
//     // This may be a problem since even though u64 (or whatever we end up
// using) are     // big Imagining some trading account, at one point it could
// fill up (with     // thousands of transactions per day)
//
//     /// Updates single field in account struct without updating it's hash.
//     /// Unsafe to use alone (hash should be recalculated).
//     /// Used only in batch updates to improve speed by reducing unnecesary
// hash     /// calculations. Returns error if update fails.
//     fn update_single_field_no_hash(&mut self, value: AccountField) ->
// Result<(), VrrbDbError> {         match value {
//             AccountField::Credits(credits) => match
// self.credits.checked_add(credits) {                 Some(new_amount) =>
// self.credits = new_amount,                 None => return
// Err(VrrbDbError::UpdateFailed(value)),             },
//             AccountField::Debits(debits) => match
// self.debits.checked_add(debits) {                 Some(new_amount) => {
//                     if self.credits >= new_amount {
//                         self.debits = new_amount
//                     } else {
//                         return Err(VrrbDbError::UpdateFailed(value));
//                     }
//                 },
//                 None => return Err(VrrbDbError::UpdateFailed(value)),
//             },
//
//             // Should the storage be impossible to delete?
//             AccountField::Storage(storage) => {
//                 self.storage = storage;
//             },
//
//             // Should the code be impossible to delete?
//             AccountField::Code(code) => {
//                 self.code = code;
//             },
//         }
//         Ok(())
//     }
//
//     /// Updates single field of the struct. Doesn't update the nonce.
//     /// Before trying to update account in database with this account, nonce
//     /// should be bumped. Finaly recalculates and updates the hash. Might
//     /// return an error.
//     ///
//     /// # Arguments:
//     /// * `update` - An AccountField enum specifying which field update (with
//     ///   what value)
//     ///
//     /// # Examples:
//     /// ```
//     /// use lrdb::{Account, AccountField};
//     /// let mut account = Account::new();
//     /// account.update_field(AccountField::Credits(300));
//     /// account.bump_nonce();
//     ///
//     /// assert_eq!(account.credits, 300);
//     /// ```
//     pub fn update_field(&mut self, update: AccountField) -> Result<(),
// VrrbDbError> {         let res = self.update_single_field_no_hash(update);
//         self.update_hash();
//         res
//     }
//
//     /// Updates all fields of the account struct accoring to supplied
//     /// AccountFieldsUpdate struct. Requires provided nonce (update's nonce)
//     /// to be exactly one number higher than accounts nonce. Recalculates
//     /// hash. Might return an error.
//     ///
//     /// # Arguments:
//     /// * `update` - An AccountFieldsUpdate struct containing instructions to
//     ///   update each field of the account struct.
//     ///
//     /// # Example:
//     /// ```
//     /// use lrdb::{Account, AccountFieldsUpdate};
//     ///
//     /// let mut account = Account::new();
//     /// let update = AccountFieldsUpdate {
//     ///     nonce: account.nonce + 1,
//     ///     credits: Some(32),
//     ///     debits: None,
//     ///     storage: None,
//     ///     code: Some(Some("Some code".to_string())),
//     /// };
//     ///
//     /// account.update(update);
//     ///
//     /// assert_eq!(account.credits, 32);
//     /// assert_eq!(account.code, Some("Some code".to_string()));
//     /// ```
//     pub fn update(&mut self, update: AccountFieldsUpdate) -> Result<(),
// VrrbDbError> {         if self.nonce + 1 != update.nonce {
//             return Err(VrrbDbError::InvalidUpdateNonce(self.nonce,
// update.nonce));         }
//         if let Some(credits_update) = update.credits {
//
// self.update_single_field_no_hash(AccountField::Credits(credits_update))?;
//         }
//         if let Some(debits_update) = update.debits {
//
// self.update_single_field_no_hash(AccountField::Debits(debits_update))?;
//         }
//         if let Some(code_update) = update.code {
//
// self.update_single_field_no_hash(AccountField::Code(code_update))?;         }
//         if let Some(storage_update) = update.storage {
//
// self.update_single_field_no_hash(AccountField::Storage(storage_update))?;
//         }
//
//         self.bump_nonce();
//         self.update_hash();
//         Ok(())
//     }
//
//     pub fn bump_nonce(&mut self) {
//         self.nonce += 1;
//     }
// }
