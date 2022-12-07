use core::cmp::Ordering;
use primitives::types::PublicKey;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, AccountError>;

pub type Nonce = u32;

/// Memory space reserved for data storage associated with the account
pub type AccountStorage = Option<String>;

/// Represents the serialized code portion of smart contract accounts
pub type Code = Option<String>;

#[derive(Debug, PartialEq, Eq, Error)]
pub enum AccountError {
    #[error("invalid update nonce: {0} {1} ")]
    InvalidUpdateNonce(u32, u32),
    #[error("failed to update {0:?}")]
    UpdateFailed(AccountField),
}

/// Stores information about given account.
#[derive(Clone, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Account {
    pub hash: String,
    pub nonce: Nonce,
    pub credits: u128,
    pub debits: u128,
    pub storage: Option<String>,
    pub code: Option<String>,
}

impl Account {
    /// Returns new, empty account.
    pub fn new() -> Account {
        let nonce = 0u32;
        let credits = 0u128;
        let debits = 0u128;
        let storage = None;
        let code = None;

        let mut hasher = Sha256::new();
        hasher.update(nonce.to_be_bytes());
        hasher.update(credits.to_be_bytes());
        hasher.update(debits.to_be_bytes());

        let hash = format!("{:x}", hasher.finalize());

        Account {
            hash,
            nonce,
            credits,
            debits,
            storage,
            code,
        }
    }

    /// Modifies accounts hash, recalculating it using account's fields.
    fn update_hash(&mut self) {
        let mut hasher = Sha256::new();
        hasher.update(self.nonce.to_be_bytes());
        hasher.update(self.credits.to_be_bytes());
        hasher.update(self.debits.to_be_bytes());

        if let Some(storage) = &self.storage {
            hasher.update(storage.as_bytes());
        }

        if let Some(code) = &self.code {
            hasher.update(code.as_bytes());
        }
        self.hash = format!("{:x}", hasher.finalize());
    }

    // TODO: do those safely
    // Should we rollback credits and debits on overflow?
    // e.g.
    // self.credits -= self.debits;
    // self.debits -= self.debits;
    //
    // This way overall balance stays the same
    // But the numbers are fine
    // This may be a problem since even though u64 (or whatever we end up using) are
    // big Imagining some trading account, at one point it could fill up (with
    // thousands of transactions per day)

    /// Updates single field in account struct without updating it's hash.
    /// Unsafe to use alone (hash should be recalculated).
    /// Used only in batch updates to improve speed by reducing unnecesary hash
    /// calculations. Returns error if update fails.
    fn update_single_field_no_hash(&mut self, value: AccountField) -> Result<()> {
        match value {
            AccountField::Credits(credits) => match self.credits.checked_add(credits) {
                Some(new_amount) => self.credits = new_amount,
                None => return Err(AccountError::UpdateFailed(value)),
            },
            AccountField::Debits(debits) => match self.debits.checked_add(debits) {
                Some(new_amount) => {
                    if self.credits >= new_amount {
                        self.debits = new_amount
                    } else {
                        return Err(AccountError::UpdateFailed(value));
                    }
                },
                None => return Err(AccountError::UpdateFailed(value)),
            },

            // Should the storage be impossible to delete?
            // TODO: reconsider storage overwrites
            AccountField::Storage(storage) => {
                self.storage = storage;
            },

            // Should the code be impossible to delete?
            AccountField::Code(code) => {
                // TODO: reconsider code overwrites
                self.code = code;
            },
        }
        Ok(())
    }

    /// Updates single field of the struct. Doesn't update the nonce.
    /// Before trying to update account in database with this account, nonce
    /// should be bumped. Finaly recalculates and updates the hash. Might
    /// return an error.
    ///
    /// # Arguments:
    /// * `update` - An AccountField enum specifying which field update (with
    ///   what value)
    ///
    pub fn update_field(&mut self, update: AccountField) -> Result<()> {
        let res = self.update_single_field_no_hash(update);
        self.update_hash();
        res
    }

    /// Updates all fields of the account struct accoring to supplied
    /// AccountFieldsUpdate struct. Requires provided nonce (update's nonce)
    /// to be exactly one number higher than accounts nonce. Recalculates
    /// hash. Might return an error.
    ///
    /// # Arguments:
    /// * `update` - An AccountFieldsUpdate struct containing instructions to
    ///   update each field of the account struct.
    ///
    pub fn update(&mut self, update: AccountFieldsUpdate) -> Result<()> {
        if self.nonce + 1 != update.nonce {
            return Err(AccountError::InvalidUpdateNonce(self.nonce, update.nonce));
        }
        if let Some(credits_update) = update.credits {
            self.update_single_field_no_hash(AccountField::Credits(credits_update))?;
        }
        if let Some(debits_update) = update.debits {
            self.update_single_field_no_hash(AccountField::Debits(debits_update))?;
        }
        if let Some(code_update) = update.code {
            self.update_single_field_no_hash(AccountField::Code(code_update))?;
        }
        if let Some(storage_update) = update.storage {
            self.update_single_field_no_hash(AccountField::Storage(storage_update))?;
        }

        self.bump_nonce();
        self.update_hash();
        Ok(())
    }

    pub fn bump_nonce(&mut self) {
        self.nonce += 1;
    }
}

/// Enum containing options for updates - used to update value of single field
/// in account struct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountField {
    Credits(u128),
    Debits(u128),
    Storage(Option<String>),
    Code(Option<String>),
}

/// Struct representing the desired updates to be applied to account.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct AccountFieldsUpdate {
    pub nonce: u32,
    pub credits: Option<u128>,
    pub debits: Option<u128>,
    pub storage: Option<Option<String>>,
    pub code: Option<Option<String>>,
}

// The AccountFieldsUpdate will be compared by `nonce`. This way the updates can
// be properly scheduled.
impl Ord for AccountFieldsUpdate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.nonce.cmp(&other.nonce)
    }
}

impl PartialOrd for AccountFieldsUpdate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
