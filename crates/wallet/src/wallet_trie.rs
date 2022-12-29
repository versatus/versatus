#[deprecated(note = "lr_trie should be used instead of this crate")]
pub mod error;

use std::{fmt::Debug, result::Result as StdResult, sync::Arc};

use error::WalletTrieError;
use keccak_hash::H256;
use left_right::ReadHandleFactory;
use lr_trie::LeftRightTrie;
use lrdb::Account;
use patriecia::{db::Database, inner::InnerTrie};
type Result<T> = StdResult<T, WalletTrieError>;

pub struct WalletTrie<D: Database> {
    wallet_trie: LeftRightTrie<D>,
}

#[deprecated(note = "Use lr_trie directly instead")]
impl<D: Database> WalletTrie<D> {
    /// Creates a new empty Wallet trie.
    pub fn new(db: Arc<D>) -> Self {
        Self {
            wallet_trie: LeftRightTrie::new(db),
        }
    }

    /// Returns read handle factory to underlying
    pub fn factory(&self) -> ReadHandleFactory<InnerTrie<D>> {
        self.wallet_trie.factory()
    }

    /// Adds a single leaf value serialized to bytes
    /// Example:
    /// ```
    /// use std::sync::Arc;
    ///
    /// use lrdb::Account;
    /// use patriecia::db::MemoryDB;
    /// use wallet_trie::WalletTrie;
    /// let memdb = Arc::new(MemoryDB::new(true));
    /// let mut wallet_trie = WalletTrie::new(memdb);
    ///
    /// Wallet_trie
    ///     .add(b"greetings.to_vec()".to_vec(), Account::new())
    ///     .unwrap();
    ///
    /// Wallet_trie
    ///     .add(b"greetings.to_vec()".to_vec(), Account::new())
    ///     .unwrap();
    ///
    /// assert_eq!(Wallet_trie.len(), 1);
    /// ```
    // TODO: Maybe it would be good idea to have both this and `trie.add` return
    // value Add tests to err
    //ket is pUblickKey?
    pub fn add(&mut self, key: Vec<u8>, wallet: Account, addrs: Vec<Account>, tokens: Vec<Account>) -> Result<()> {
        //implement comparison fxns?
        let wallet_addr = HashMap::new();
        let addr_token = HashMap::new();
        let tokens_and_bals: HashMap<Account, u128> = tokens.clone().iter().map(|token| (token, (token.credit - token.debit))).collect();

        //check that tokens_and_bals.len() == tokens.len()

        wallet_addr.insert(wallet, addr_token.insert(addrs, tokens_and_bals));

        match bincode::serialize(&wallet_addr) {
            Ok(serialized) => {
                self.wallet_trie.add(key, serialized);
                return Ok(());
            },
            Err(_) => return Err(WalletTrieError::FailedToSerializeAccount(wallet_addr)),
        }
    }

    /// Extends the Wallet trie with the provided iterator over leaf values as
    /// bytes. Example:
    /// ```
    /// use std::sync::Arc;
    ///
    /// use lr_trie::Bytes;
    /// use patriecia::db::MemoryDB;
    /// use Wallet_trie::WalletTrie;
    ///
    /// let memdb = Arc::new(MemoryDB::new(true));
    /// let mut Wallet_trie = WalletTrie::new(memdb);
    ///
    /// let vals: Vec<(Vec<u8>, Vec<u8>)> = vec![
    ///     (b"abcdefg".to_vec(), b"abcdefg".to_vec()),
    ///     (b"hijkl".to_vec(), b"hijkl".to_vec()),
    ///     (b"mnopq".to_vec(), b"mnopq".to_vec()),
    /// ];
    ///
    /// Wallet_trie.extend(vals);
    /// assert_eq!(Wallet_trie.len(), 3);
    /// ```
    pub fn extend(&mut self, values: Vec<(Vec<u8>, Vec<u8>)>) {
        self.wallet_trie.extend(values);
    }

    /// Returns the trie's Merkle root.
    /// Example:
    /// ```
    /// use std::sync::Arc;
    ///
    /// use lr_trie::Bytes;
    /// use patriecia::db::MemoryDB;
    /// use Wallet_trie::WalletTrie;
    ///
    /// let memdb = Arc::new(MemoryDB::new(true));
    /// let mut Wallet_trie_a = WalletTrie::new(memdb);
    ///
    /// let memdb = Arc::new(MemoryDB::new(true));
    /// let mut Wallet_trie_b = WalletTrie::new(memdb);
    ///
    /// let vals: Vec<(Vec<u8>, Vec<u8>)> = vec![
    ///     (b"abcdefg".to_vec(), b"abcdefg".to_vec()),
    ///     (b"hijkl".to_vec(), b"hijkl".to_vec()),
    ///     (b"mnopq".to_vec(), b"mnopq".to_vec()),
    /// ];
    ///
    /// Wallet_trie_a.extend(vals.clone());
    /// Wallet_trie_b.extend(vals.clone());
    ///
    /// Wallet_trie_a.extend(vals.clone());
    /// Wallet_trie_b.extend(vals.clone());
    ///
    /// assert_eq!(Wallet_trie_a.root(), Wallet_trie_b.root());
    /// ```
    pub fn root(&self) -> Option<H256> {
        self.wallet_trie.root()
    }

    /// Returns the count of leaves in the Wallet trie.
    /// Example:
    /// ```
    /// use std::sync::Arc;
    ///
    /// use lr_trie::Bytes;
    /// use patriecia::db::MemoryDB;
    /// use Wallet_trie::WalletTrie;
    ///
    /// let memdb = Arc::new(MemoryDB::new(true));
    /// let mut Wallet_trie = WalletTrie::new(memdb);
    ///
    /// let vals: Vec<(Vec<u8>, Vec<u8>)> = vec![
    ///     (b"abcdefg".to_vec(), b"abcdefg".to_vec()),
    ///     (b"hijkl".to_vec(), b"hijkl".to_vec()),
    ///     (b"mnopq".to_vec(), b"mnopq".to_vec()),
    /// ];
    ///
    /// Wallet_trie.extend(vals.clone());
    ///
    /// Wallet_trie.extend(vals.clone());
    ///
    /// assert_eq!(Wallet_trie.len(), 3);
    /// ```
    pub fn len(&self) -> usize {
        self.wallet_trie.len()
    }

    /// Returns true if there are no values in the trie.
    /// Example:
    /// ```
    /// use std::sync::Arc;
    ///
    /// use patriecia::db::MemoryDB;
    /// use Wallet_trie::WalletTrie;
    ///
    /// let memdb = Arc::new(MemoryDB::new(true));
    /// let mut Wallet_trie = WalletTrie::new(memdb);
    ///
    /// let memdb = Arc::new(MemoryDB::new(true));
    /// let mut Wallet_trie = WalletTrie::new(memdb);
    ///
    /// assert_eq!(Wallet_trie.len(), 0);
    /// ```
    pub fn is_empty(&self) -> bool {
        self.wallet_trie.len() == 0
    }
}

impl<D: Database> PartialEq for WalletTrie<D> {
    fn eq(&self, other: &Self) -> bool {
        self.root() == other.root()
    }
}

impl<D: Database> Debug for WalletTrie<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WalletTrie")
            .field("trie", &self.trie)
            .finish()
    }
}


#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use lrdb::Account;
    use patriecia::{db::MemoryDB, trie::Trie};

    use super::*;

    #[test]
    fn new_creates_default_empty_trie() {
        let memdb = Arc::new(MemoryDB::new(true));
        let Wallet_trie = WalletTrie::new(memdb);

        assert!(Wallet_trie.root().is_some());
        assert_eq!(Wallet_trie.len(), 0);
    }

    #[test]
    #[ignore = "breaking changes introduced to lr_trie make this test fail"]
    fn new_creates_trie_from_lrdb_values() {
        let memdb = Arc::new(MemoryDB::new(true));
        let mut Wallet_trie = WalletTrie::new(memdb);

        Wallet_trie.add(b"abcdefg".to_vec(), Account::new()).unwrap();
        Wallet_trie.add(b"hijkl".to_vec(), Account::new()).unwrap();
        Wallet_trie.add(b"mnopq".to_vec(), Account::new()).unwrap();

        let root = Wallet_trie.root().unwrap();
        let root = format!("0x{}", hex::encode(root));

        let target_root =
            "0xb932b90dadf9a1f3c54c89f112f0d2c969753b20c112a98802d349d1db2859e0".to_string();

        let read_handle = Wallet_trie.trie.handle();

        let default_account = bincode::serialize(&Account::new()).unwrap();
        let read_value = read_handle.get(b"abcdefg").unwrap().unwrap();

        assert_eq!(
            Wallet_trie.trie.get().get(b"abcdefg").unwrap().unwrap(),
            default_account
        );

        assert_eq!(
            Wallet_trie.trie.get().get(b"hijkl").unwrap().unwrap(),
            default_account
        );

        assert_eq!(
            Wallet_trie.trie.get().get(b"mnopq").unwrap().unwrap(),
            default_account
        );

        assert_eq!(root, target_root);
    }

    #[test]
    fn should_add_node_to_trie() {
        let memdb = Arc::new(MemoryDB::new(true));
        let mut Wallet_trie = WalletTrie::new(memdb);

        assert!(Wallet_trie.root().is_some());
        assert_eq!(Wallet_trie.len(), 0);

        Wallet_trie
            .add(b"greetings".to_vec(), Account::new())
            .unwrap();

        assert_ne!(Wallet_trie.root(), None);
        assert_eq!(Wallet_trie.len(), 1);
    }

    #[test]
    fn should_extend_trie_with_nodes() {
        let memdb = Arc::new(MemoryDB::new(true));
        let mut Wallet_trie = WalletTrie::new(memdb);

        assert!(Wallet_trie.root().is_some());
        assert_eq!(Wallet_trie.len(), 0);

        let vals: Vec<(Vec<u8>, Vec<u8>)> = vec![
            (b"abcdefg".to_vec(), b"abcdefg".to_vec()),
            (b"hijkl".to_vec(), b"hijkl".to_vec()),
            (b"mnopq".to_vec(), b"mnopq".to_vec()),
        ];

        Wallet_trie.extend(vals);

        assert_ne!(Wallet_trie.root(), None);
        assert_eq!(Wallet_trie.len(), 3);
    }

    #[test]
    fn should_return_true_if_root_is_equal_to_other_trie_root() {
        let memdb = Arc::new(MemoryDB::new(true));

        let mut Wallet_trie_a = WalletTrie::new(memdb.clone());
        let mut Wallet_trie_b = WalletTrie::new(memdb.clone());

        let vals: Vec<(Vec<u8>, Vec<u8>)> = vec![
            (b"abcdefg".to_vec(), b"abcdefg".to_vec()),
            (b"hijkl".to_vec(), b"hijkl".to_vec()),
            (b"mnopq".to_vec(), b"mnopq".to_vec()),
        ];

        Wallet_trie_a.extend(vals.clone());
        Wallet_trie_b.extend(vals.clone());

        assert_eq!(Wallet_trie_a, Wallet_trie_b);
    }

    #[test]
    fn should_return_false_if_root_is_not_equal_to_other_trie_root() {
        let memdb = Arc::new(MemoryDB::new(true));

        let mut Wallet_trie_a = WalletTrie::new(memdb.clone());
        let mut Wallet_trie_b = WalletTrie::new(memdb.clone());

        let vals: Vec<(Vec<u8>, Vec<u8>)> = vec![
            (b"abcdefg".to_vec(), b"abcdefg".to_vec()),
            (b"hijkl".to_vec(), b"hijkl".to_vec()),
            (b"mnopq".to_vec(), b"mnopq".to_vec()),
        ];

        Wallet_trie_a.extend(vals.clone());
        Wallet_trie_b.extend(vals.clone());
        Wallet_trie_b.add(b"mnopq".to_vec(), Account::new()).unwrap();

        assert_ne!(Wallet_trie_a, Wallet_trie_b);
    }
}
