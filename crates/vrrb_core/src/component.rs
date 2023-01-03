// Feature Tag(s): Validator Stake Calculation, Tx Validation, Tx Confirmation,
// Masternode Signing, Masternode Election Node Reputation Scores, Block
// Structure, Packet Processing, Message Processing, Message Allocating, Message
// Caching
use serde::{Deserialize, Serialize};

/// Basic Command Constants
//TODO: Need to add all potential input commands
pub const SENDTXN: &str = "SENDTXN";
pub const GETBAL: &str = "GETBAL";
pub const GETSTATE: &str = "GETSTATE";
pub const MINEBLOCK: &str = "MINEBLK";
pub const SENDADDRESS: &str = "SENDADR";
pub const STOPMINE: &str = "STOPMINE";
pub const GETHEIGHT: &str = "GETHEIGHT";
pub const STOP: &str = "STOP";

/// Component Types of a state update
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ComponentTypes {
    Genesis,
    Child,
    Parent,
    Blockchain,
    Ledger,
    NetworkState,
    Archive,
    All,
}

impl std::hash::Hash for ComponentTypes {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
    }
}

impl ComponentTypes {
    /// Converts a Componenet type into an integer.
    pub fn to_int(&self) -> u8 {
        match *self {
            ComponentTypes::Genesis => 0,
            ComponentTypes::Child => 1,
            ComponentTypes::Parent => 2,
            ComponentTypes::NetworkState => 3,
            ComponentTypes::Ledger => 4,
            ComponentTypes::Blockchain => 5,
            ComponentTypes::Archive => 6,
            ComponentTypes::All => 7,
        }
    }
}

impl PartialEq for ComponentTypes {
    fn eq(&self, other: &Self) -> bool {
        self.to_int() == other.to_int()
    }
}

impl Eq for ComponentTypes {}
