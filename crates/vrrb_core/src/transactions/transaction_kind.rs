use serde::{Deserialize, Serialize};


#[derive(Debug, Deserialize, Clone, Serialize, Eq, PartialEq)]
pub enum TransactionKind {
    Transfer,
}