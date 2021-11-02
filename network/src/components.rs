use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum StateComponent {
    All,
    Ledger,
    NetworkState,
    Blockchain,
    Archive,
}

impl StateComponent {
    pub fn as_bytes(&self) -> Vec<u8> {
        serde_json::to_string(self).unwrap().as_bytes().to_vec()
    }

    pub fn from_bytes(data: &[u8]) -> StateComponent {
        serde_json::from_slice::<StateComponent>(data).unwrap()
    }
}