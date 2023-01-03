use ritelinked::LinkedHashMap;
use serde::{Deserialize, Serialize};

use crate::{claim::Claim, nonceable::Nonceable, ownable::Ownable};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Ledger<C: Clone + Ownable + Nonceable + Serialize> {
    pub credits: LinkedHashMap<String, u128>,
    pub debits: LinkedHashMap<String, u128>,
    pub claims: LinkedHashMap<String, C>,
}

impl Default for Ledger<Claim> {
    fn default() -> Self {
        Self::new()
    }
}

impl Ledger<Claim> {
    pub fn new() -> Ledger<Claim> {
        Ledger {
            credits: LinkedHashMap::new(),
            debits: LinkedHashMap::new(),
            claims: LinkedHashMap::new(),
        }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_string().as_bytes().to_vec()
    }

    pub fn from_bytes(data: Vec<u8>) -> Ledger<Claim> {
        serde_json::from_slice::<Ledger<Claim>>(&data).unwrap()
    }

    // TODO: Should we change the name?
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn from_string(string: String) -> Ledger<Claim> {
        serde_json::from_str::<Ledger<Claim>>(&string).unwrap()
    }
}
