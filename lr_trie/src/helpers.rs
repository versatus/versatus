use crate::db::Database as DbTrait;

#[derive(Debug, thiserror::Error)]
pub enum TrieDbError {}
pub trait Database: Clone + Default + AsMut<dyn DbTrait<Error = TrieDbError>> {}
