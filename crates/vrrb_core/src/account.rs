use std::cmp::Ordering;

use primitives::{
    {PublicKey, SerializedPublicKey},
    AccountKeypair,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{Error, Result};

/// Enum containing options for updates - used to update value of single field
/// in account struct.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum AccountField {
    Credits(u128),
    Debits(u128),
    Storage(Option<String>),
    Code(Option<String>),
}

/// Struct representing the desired updates to be applied to account.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct UpdateArgs {
    pub nonce: u32,
    pub credits: Option<u128>,
    pub debits: Option<u128>,
    pub storage: Option<Option<String>>,
    pub code: Option<Option<String>>,
}

// The AccountFieldsUpdate will be compared by `nonce`. This way the updates can
// be properly scheduled.
impl Ord for UpdateArgs {
    fn cmp(&self, other: &Self) -> Ordering {
        self.nonce.cmp(&other.nonce)
    }
}

impl PartialOrd for UpdateArgs {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub type AccountNonce = u32;

#[derive(Clone, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Account {
    pub hash: String,
    pub nonce: AccountNonce,
    pub credits: u128,
    pub debits: u128,
    pub storage: Option<String>,
    pub code: Option<String>,
    pub pubkey: SerializedPublicKey,
}

impl Account {
    /// Returns new, empty account.
    pub fn new(pubkey: secp256k1::PublicKey) -> Account {
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

        let pubkey = pubkey.serialize().to_vec();

        Account {
            hash,
            nonce,
            credits,
            debits,
            storage,
            code,
            pubkey,
        }
    }

    /// Modifies accounts hash, recalculating it using account's fields.
    fn rehash(&mut self) {
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
                None => return Err(Error::Other(format!("failed to update {value:?}"))),
            },
            AccountField::Debits(debits) => match self.debits.checked_add(debits) {
                Some(new_amount) => {
                    if self.credits >= new_amount {
                        self.debits = new_amount
                    } else {
                        return Err(Error::Other(format!("failed to update {value:?}")));
                    }
                },
                None => return Err(Error::Other(format!("failed to update {value:?}"))),
            },

            // Should the storage be impossible to delete?
            AccountField::Storage(storage) => {
                self.storage = storage;
            },

            // Should the code be impossible to delete?
            AccountField::Code(code) => {
                self.code = code;
            },
        }
        Ok(())
    }

    /// Updates single field of the struct. Doesn't update the nonce.
    /// Before trying to update account in database with this account, nonce
    /// should be bumped. Finaly recalculates and updates the hash. Might
    /// return an error.
    pub fn update_field(&mut self, update: AccountField) -> Result<()> {
        let res = self.update_single_field_no_hash(update);
        self.rehash();
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
    pub fn update(&mut self, args: UpdateArgs) -> Result<()> {
        if self.nonce + 1 != args.nonce {
            return Err(Error::Other(format!(
                "nonce from args {} is smaller than current nonce {}",
                args.nonce, self.nonce
            )));
        }
        if let Some(credits_update) = args.credits {
            self.update_single_field_no_hash(AccountField::Credits(credits_update))?;
        }
        if let Some(debits_update) = args.debits {
            self.update_single_field_no_hash(AccountField::Debits(debits_update))?;
        }
        if let Some(code_update) = args.code {
            self.update_single_field_no_hash(AccountField::Code(code_update))?;
        }
        if let Some(storage_update) = args.storage {
            self.update_single_field_no_hash(AccountField::Storage(storage_update))?;
        }

        self.bump_nonce();
        self.rehash();
        Ok(())
    }

    pub fn bump_nonce(&mut self) {
        self.nonce += 1;
    }
}

#[cfg(test)]
mod tests {
    use primitives::generate_account_keypair;

    use super::*;

    #[test]
    fn should_create_account() {
        let (_, pk) = generate_account_keypair();

        let account = Account::new(pk);

        assert_eq!(account.nonce, 0);
    }
}
