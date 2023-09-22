//! WIP storage table
//!
//! Use the same backing storage for all instances of
//! a left right trie.
//!
//! This means that we must settle on a singular type
//! to read from and write to since a single generic
//! or dyn trait type is insufficient for setting up
//! multiple tries with different types, even if they
//! all implement the same parent trait.
//!
//! By way of organizing the family of the trie in a
//! HashMap, we can categorize how each trie's KV pairs
//! are interacted with.
//!
//! This design structure also allows us to add more
//! columns as needed, simply by passing the shared
//! storage pointer to a new column.
use anyhow::Result;
use integral_db::LeftRightTrie;
use patriecia::{TreeReader, TreeWriter, VersionedDatabase};
use sha2::Sha256;
use std::{collections::HashMap, sync::Arc};

#[derive(Debug)]
pub struct StorageTable<'a, D: VersionedDatabase + TreeWriter + TreeReader> {
    /// map of table classifications and their corresponding tries.
    ///
    /// The types for the key value pairs in this circumstance must be Vec<u8>
    /// to ensure multiple types can be stored, as this structure is a roof
    /// over many possible types to be contained in a single structure. Therefore,
    /// generics won't help us since we can only declare a single type at a time.
    ///
    /// each trie contains the same pointer in memeory to the backing storage.
    map: HashMap<String, LeftRightTrie<'a, Vec<u8>, Vec<u8>, D, Sha256>>,
    /// pointer in memory to the DB, use only to create new columns
    /// never use this to manipulate data directly
    _db_ptr: Arc<D>,
}
impl<'a, D> Default for StorageTable<'a, D>
where
    D: VersionedDatabase + TreeReader + TreeWriter + Default,
{
    fn default() -> Self {
        let db = D::default();
        // TODO: add columns claims, state & transactions
        let _db_ptr = Arc::new(db);
        // give pointer to each trie

        Self {
            map: Default::default(),
            _db_ptr,
        }
    }
}
impl<'a, D> StorageTable<'a, D>
where
    D: VersionedDatabase + TreeReader + TreeWriter,
{
    /// Adds a new column to the table if it doesn't exist. If it does exist, this will
    /// overwrite the previous data.
    ///
    /// All tries point to the same backing storage by way of cloned pointers to the
    /// same place in memory.
    pub fn insert_column(&mut self, cf: &str) -> Result<()> {
        // create a clone of the Arc pointing to the same place in memoryd
        self.map
            .insert(cf.into(), LeftRightTrie::new(Arc::clone(&self._db_ptr)));
        // TODO: write method to adapter to create new cf
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use crate::{table::StorageTable, RocksDbAdapter};

    #[test]
    fn test_table_setup() {
        let table = StorageTable::<RocksDbAdapter>::default();
        dbg!(&table);
    }
}
