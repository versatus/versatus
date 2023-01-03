use std::{cmp::Ordering, collections::HashMap, hash::Hash, marker::PhantomData, time::SystemTime};

use lr_trie::LeftRightTrie;
use patriecia::db::MemoryDB;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vrrb_core::{account::Account, keypair::PublicKeys};

/// Struct representing the LeftRight Database.
///
/// `ReadHandleFactory` provides a way of creating new ReadHandles to the
/// database.
///
/// `WriteHandles` provides a way to gain write access to the database.
/// `last_refresh` denotes the lastest `refresh` of the database.
#[derive(Debug)]
#[deprecated(note = "replaced by purpose specific stores")]
pub struct LeftRightDatabase<'a, K, V>
where
    K: Clone + Eq + Hash,
    V: Clone + Eq,
{
    trie: LeftRightTrie<'a, Vec<u8>, Account, MemoryDB>,
    last_refresh: std::time::SystemTime,
    _marker: PhantomData<(&'a (), K, V)>,
}
