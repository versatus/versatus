use std::{collections::HashMap, fmt::Debug, sync::Arc};

use keccak_hash::H256;
pub use left_right::ReadHandleFactory;
use patriecia::{
    db::Database,
    inner::{InnerTrie, TrieIterator},
    trie::Trie,
};
use serde::{Deserialize, Serialize};

use crate::{LeftRightTrieError, Result};

pub type Proof = Vec<u8>;

#[derive(Debug, Clone)]
pub struct InnerTrieWrapper<D>
where
    D: Database,
{
    inner: InnerTrie<D>,
}

impl<D> InnerTrieWrapper<D>
where
    D: Database,
{
    pub fn new(inner: InnerTrie<D>) -> Self {
        Self { inner }
    }

    pub fn get<K, V>(&self, key: &K) -> Result<V>
    where
        K: for<'a> Deserialize<'a> + Serialize + Clone,
        V: for<'a> Deserialize<'a> + Serialize + Clone,
    {
        let key = bincode::serialize(key).unwrap_or_default();

        let raw_value_opt = self
            .inner
            .get(&key)
            .map_err(|err| LeftRightTrieError::Other(err.to_string()))?;

        let raw_value = raw_value_opt.ok_or_else(|| {
            LeftRightTrieError::Other("received none value from inner trie".to_string())
        })?;

        let value = bincode::deserialize::<V>(&raw_value)
            .map_err(|err| LeftRightTrieError::Other(err.to_string()))?;

        Ok(value)
    }

    pub fn contains<'a, K, V>(&self, key: &'a K) -> Result<bool>
    where
        K: Serialize + Deserialize<'a>,
        V: Serialize + Deserialize<'a>,
    {
        let key = bincode::serialize(&key).unwrap_or_default();
        self.inner
            .contains(&key)
            .map_err(|err| LeftRightTrieError::Other(err.to_string()))
    }

    pub fn insert<'a, K, V>(&mut self, key: K, value: V) -> Result<()>
    where
        K: Serialize + Deserialize<'a>,
        V: Serialize + Deserialize<'a>,
    {
        let key = bincode::serialize(&key).unwrap_or_default();
        let value = bincode::serialize(&value).unwrap_or_default();

        self.inner
            .insert(&key, &value)
            .map_err(|err| LeftRightTrieError::Other(err.to_string()))
    }

    pub fn remove<'a, K, V>(&mut self, key: K) -> Result<bool>
    where
        K: Serialize + Deserialize<'a>,
        V: Serialize + Deserialize<'a>,
    {
        let key = bincode::serialize(&key).unwrap_or_default();
        self.inner
            .remove(&key)
            .map_err(|err| LeftRightTrieError::Other(err.to_string()))
    }

    pub fn root_hash(&mut self) -> Result<H256> {
        self.commit()
    }

    pub fn get_proof<'a, K, V>(&mut self, key: K) -> Result<Vec<Vec<u8>>>
    where
        K: Serialize + Deserialize<'a>,
        V: Serialize + Deserialize<'a>,
    {
        let key = bincode::serialize(&key).unwrap_or_default();
        self.inner
            .get_proof(&key)
            .map_err(|err| LeftRightTrieError::Other(err.to_string()))
    }

    pub fn verify_proof<'a, K, V>(
        &self,
        root_hash: H256,
        key: &K,
        proof: Vec<Vec<u8>>,
    ) -> Result<Option<Proof>>
    where
        K: Serialize + Deserialize<'a>,
        V: Serialize + Deserialize<'a>,
    {
        let key = bincode::serialize(key).unwrap_or_default();

        self.inner
            .verify_proof(root_hash, &key, proof)
            .map_err(|err| LeftRightTrieError::Other(err.to_string()))
    }

    pub fn commit(&mut self) -> Result<H256> {
        self.inner
            .commit()
            .map_err(|err| LeftRightTrieError::Other(err.to_string()))
    }

    // pub fn entries<'a, K, V>(&self) -> HashMap<K, V>
    // where
    //     K: Serialize + Deserialize<'a> + std::hash::Hash + Default + Eq +
    // PartialEq,     V: Serialize + Deserialize<'a> + Default,
    // {
    //     let mut map = HashMap::new();
    //     for (k, v) in self.inner.iter() {
    //         let key = bincode::deserialize(&k).unwrap_or_default();
    //         let value = bincode::deserialize(&v).unwrap_or_default();
    //
    //         map.insert(key, value);
    //     }
    //
    //     map
    // }

    pub fn iter(&self) -> TrieIterator<D> {
        self.inner.iter()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn db(&self) -> Arc<D> {
        self.inner.db()
    }
}
