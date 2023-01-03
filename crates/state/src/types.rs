use std::path::PathBuf;

use reward::reward::Reward;

pub type StateGenesisBlock = Option<Vec<u8>>;
pub type StateChildBlock = Option<Vec<u8>>;
pub type StateParentBlock = Option<Vec<u8>>;
pub type StateBlockchain = Option<Vec<u8>>;
pub type StateLedger = Option<Vec<u8>>;
pub type StateNetworkState = Option<Vec<u8>>;
pub type StateArchive = Option<Vec<u8>>;
// pub type StatePath = String;
pub type StatePath = PathBuf;
pub type LedgerBytes = Vec<u8>;
pub type CreditsRoot = Option<String>;
pub type DebitsRoot = Option<String>;
pub type StateRewardState = Option<Reward>;
pub type StateRoot = Option<String>;
pub type CreditsHash = String;
pub type DebitsHash = String;
pub type StateHash = String;
