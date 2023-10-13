use std::{
    cmp::Ordering,
    collections::HashSet,
    fmt::Formatter,
    hash::{Hash, Hasher},
};

use primitives::Address;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::transactions::transaction::TransactionDigest;
use crate::{Error, Result};

/// Enum containing options for updates - used to update value of single field
/// in account struct.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum AccountField {
    Credits(u128),
    Debits(u128),
    Storage(Option<String>),
    PackageAddress(Option<String>),
    Digests(AccountDigests),
}

/// Wrapper to provide convenient access to all the digests
/// throughout the history of a given account, separated by whether
/// the txn was sent from the account, received by the account, or
/// was a staking transaction.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountDigests {
    sent: HashSet<TransactionDigest>,
    recv: HashSet<TransactionDigest>,
    stake: HashSet<TransactionDigest>,
    // TODO: Add withdrawaltransaction digests for
    // withdrawing stake.
}

impl AccountDigests {
    pub fn len(&self) -> usize {
        let mut len = 0;
        len += self.sent.len();
        len += self.recv.len();
        len += self.stake.len();

        len
    }

    /// Returns true if the length of [AccountDigests] combined
    /// [TransactionDigest]s is zero.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the HashSet of all `TransactionDigest`s for
    /// all transactions throughout history sent by the current
    /// account
    pub fn get_sent(&self) -> HashSet<TransactionDigest> {
        self.sent.clone()
    }

    /// Returns the HashSet of all `TransactionDigest`s for
    /// all transactions throughout history received by the current
    /// account
    pub fn get_recv(&self) -> HashSet<TransactionDigest> {
        self.recv.clone()
    }

    /// Returns the HashSet of all `TransactionDigest`s for
    /// all staking transactions throughout history by the current
    /// account
    pub fn get_stake(&self) -> HashSet<TransactionDigest> {
        self.stake.clone()
    }

    /// Given an AccountDigests struct, updates the current
    /// instance by extending each of the sets of transaction
    /// digests for any new digests.
    pub fn extend_all(&mut self, other: AccountDigests) {
        self.sent.extend(other.get_sent());
        self.recv.extend(other.get_recv());
        self.stake.extend(other.get_stake());
    }

    /// Inserts a transaction digest into the `sent` set of
    /// transaction digests
    pub fn insert_sent(&mut self, digest: TransactionDigest) {
        self.sent.insert(digest);
    }

    /// Inserts a transaction digest into the `recv` set of
    /// transaction digests
    pub fn insert_recv(&mut self, digest: TransactionDigest) {
        self.recv.insert(digest);
    }

    /// Inserts a transaction digest into the `stake` set of
    /// transaction digests
    pub fn insert_stake(&mut self, digest: TransactionDigest) {
        self.stake.insert(digest);
    }

    /// Takes a generic Iterator of AccountDigests and consolidates them
    /// and extends all of the sets in the current instance for each
    /// AccountDigests struct in the Iterator
    pub fn consolidate<I: Iterator<Item = AccountDigests>>(&mut self, others: I) {
        others.for_each(|other| self.extend_all(other))
    }
}

/// Produces an empty AccountDigests instance
impl Default for AccountDigests {
    fn default() -> Self {
        AccountDigests {
            sent: HashSet::new(),
            recv: HashSet::new(),
            stake: HashSet::new(),
        }
    }
}

/// Struct representing the desired updates to be applied to account.
/// TODO: impl Default for UpdateArgs { ... }
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct UpdateArgs {
    pub address: Address,
    pub nonce: Option<u128>,
    pub credits: Option<u128>,
    pub debits: Option<u128>,
    pub storage: Option<Option<String>>,
    pub package_address: Option<Option<String>>,
    pub digests: Option<AccountDigests>,
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

#[allow(clippy::derived_hash_with_manual_eq)]
impl Hash for UpdateArgs {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.address.hash(state);
        self.nonce.hash(state);
        self.credits.hash(state);
        self.debits.hash(state);
        self.storage.hash(state);
        self.package_address.hash(state);

        if let Some(ref digests) = self.digests {
            digests.len().hash(state); // Hash the number of digests
            let mut consolidated_digests = digests.get_sent();
            consolidated_digests.extend(digests.get_recv());
            consolidated_digests.extend(digests.get_stake());
            let mut sorted_digests: Vec<TransactionDigest> =
                { consolidated_digests.into_iter().collect() };
            sorted_digests.sort_unstable(); // Sort digests by keys to ensure consistent hash order
            for value in sorted_digests {
                value.hash(state);
            }
        } else {
            0u8.hash(state);
        }
    }
}

pub type AccountNonce = u128;

#[derive(Clone, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Account {
    address: Address,
    hash: String,
    nonce: AccountNonce,
    credits: u128,
    debits: u128,
    storage: Option<String>,
    package_address: Option<String>,
    digests: AccountDigests,
    // #[serde(skip_serializing)]
    // created_at: i64,
    // #[serde(skip_serializing)]
    // updated_at: Option<i64>,
}

impl Account {
    /// Returns new, empty account.
    pub fn new(address: Address) -> Account {
        let nonce = 0u128;
        let credits = 0u128;
        let debits = 0u128;
        let storage = None;
        let package_address = None;
        let digests = AccountDigests::default();

        let mut hasher = Sha256::new();
        hasher.update(nonce.to_be_bytes());
        hasher.update(credits.to_be_bytes());
        hasher.update(debits.to_be_bytes());

        let hash = format!("{:x}", hasher.finalize());

        Account {
            address,
            hash,
            nonce,
            credits,
            debits,
            storage,
            package_address,
            digests,
            // created_at: Utc::now().timestamp(),
            // updated_at: None,
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

        if let Some(package_address) = &self.package_address {
            hasher.update(package_address.as_bytes());
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
    //
    // THOUGHT:
    //
    // WRT the above, maybe what we want to do is use bytes/hex strings instead of
    // values and then just do byte/hex math for display...

    /// Updates single field in account struct without updating it's hash.
    /// Unsafe to use alone (hash should be recalculated).
    /// Used only in batch updates to improve speed by reducing unnecesary hash
    /// calculations. Returns error if update fails.
    fn update_single_field_no_hash(&mut self, value: AccountField) -> Result<()> {
        match value {
            AccountField::Credits(credits) => match self.credits.checked_add(credits) {
                Some(new_amount) => {
                    self.credits = new_amount;
                },
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
            AccountField::PackageAddress(package_address) => {
                self.package_address = package_address;
            },

            // Maybe we want to change `digests` to digest and only
            // update one at a time, though this could become a problem
            // if a single account has multiple transactions per round
            // better to batch them and update or at least have option to.
            AccountField::Digests(digests) => {
                self.digests.extend_all(digests);
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
        if let Some(nonce) = args.nonce {
            if nonce < self.nonce {
                return Err(Error::Other(format!(
                    "nonce from args {} is smaller than current nonce {}",
                    nonce, self.nonce
                )));
            } else if nonce > self.nonce + 1 {
                self.update_nonce(nonce);
            } else {
                self.bump_nonce();
            }
        }

        if let Some(credits_update) = args.credits {
            self.update_single_field_no_hash(AccountField::Credits(credits_update))?;
        }

        if let Some(debits_update) = args.debits {
            self.update_single_field_no_hash(AccountField::Debits(debits_update))?;
        }

        if let Some(code_update) = args.package_address {
            self.update_single_field_no_hash(AccountField::PackageAddress(code_update))?;
        }
        if let Some(storage_update) = args.storage {
            self.update_single_field_no_hash(AccountField::Storage(storage_update))?;
        }

        if let Some(digests) = args.digests {
            self.update_single_field_no_hash(AccountField::Digests(digests))?;
        }

        // self.updated_at = Some(Utc::now().timestamp());
        self.rehash();
        Ok(())
    }

    /// Increments the current account nonce by 1
    pub fn bump_nonce(&mut self) {
        self.nonce += 1;
    }

    /// Batch updates nonce instead of simply incrementing
    /// this allows us to more efficiently update an account
    /// when a given account has multiple `sends` in a given
    /// round.
    fn update_nonce(&mut self, nonce: AccountNonce) {
        self.nonce = nonce;
    }

    pub fn address(&self) -> &Address {
        &self.address
    }
    pub fn hash(&self) -> &str {
        &self.hash
    }
    pub fn nonce(&self) -> AccountNonce {
        self.nonce
    }
    pub fn credits(&self) -> u128 {
        self.credits
    }

    pub fn set_credits(&mut self, credits: u128) {
        self.credits = credits;
    }

    pub fn debits(&self) -> u128 {
        self.debits
    }
    pub fn storage(&self) -> &Option<String> {
        &self.storage
    }
    pub fn package_address(&self) -> &Option<String> {
        &self.package_address
    }
    pub fn digests(&self) -> &AccountDigests {
        &self.digests
    }
    // pub fn created_at(&self) -> i64 {
    //     self.created_at
    // }
    // pub fn updated_at(&self) -> Option<i64> {
    //     self.updated_at
    // }
}

impl std::fmt::Display for Account {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let serialized_account = serde_json::to_string_pretty(self).map_err(|_| std::fmt::Error)?;
        write!(f, "{}", serialized_account)
    }
}

#[cfg(test)]
mod tests {
    use primitives::generate_account_keypair;

    use super::*;

    #[test]
    fn should_create_account() {
        let (_, pk) = generate_account_keypair();
        let address = Address::new(pk);

        let account = Account::new(address);

        assert_eq!(account.nonce, 0);
    }
}
