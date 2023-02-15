/// The unit of time within VRRB.
/// It lasts for some number
pub type Epoch = u128;

pub const GENESIS_EPOCH: Epoch = 0;
pub const GROSS_UTILITY_PERCENTAGE: f64 = 0.01;
pub const PERCENTAGE_CHANGE_SUPPLY_CAP: f64 = 0.25;

// Time-related helper constants
pub const NANO: u128 = 1;
pub const MICRO: u128 = NANO * 1000;
pub const MILLI: u128 = MICRO * 1000;
pub const SECOND: u128 = MILLI * 1000;
pub const VALIDATOR_THRESHOLD: f64 = 0.60;

pub const NUMBER_OF_NETWORK_PACKETS: usize = 32;
pub const DEFAULT_VRRB_DATA_DIR_PATH: &str = ".vrrb";
pub const DEFAULT_VRRB_DB_PATH: &str = ".vrrb/node/node/db";

pub type ByteVec = Vec<u8>;
pub type ByteSlice<'a> = &'a [u8];

type Hash = Vec<u8>;
pub type TxHash = Vec<u8>;
pub type TxHashString = String;
pub type PayloadHash = Hash;
pub type BlockHash = Hash;
pub type RawSignature = Vec<u8>;
pub type PeerId = Vec<u8>;
