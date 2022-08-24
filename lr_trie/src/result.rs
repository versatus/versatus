use crate::error::TrieError;
use std::result::Result as StdResult;

pub type Result<T> = StdResult<T, TrieError>;
