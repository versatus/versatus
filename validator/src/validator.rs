//FEATURE TAG(S): Validator Cores, Tx Validation, Tx Writes to Confirmed, Block Validation & Confirmation
//TODO: Rebuild this entire module.
#![allow(unused_imports)]
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::VecDeque;
use txn::txn::Txn; 
use pool::pool::Pool;
use state::state::NetworkState;
use verifiable::verifiable::Verifiable;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxnValidator {
    pub pubkey: String,
    pub vote: bool,
    pub txn: Txn,
}

impl TxnValidator {
    pub fn new(
        pubkey: String,
        txn: Txn,
        network_state: &NetworkState,
        txn_pool: &Pool<String, Txn>,
    ) -> TxnValidator {
        let vote = {
            if let Ok(true) = txn.clone().valid(&None, &(network_state.to_owned(), txn_pool.to_owned())) {
                true
            } else {
                false
            }
        };
        TxnValidator {
            pubkey,
            vote,
            txn,
        }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        serde_json::to_string(self).unwrap().as_bytes().to_vec()
    }

    pub fn from_bytes(data: &[u8]) -> TxnValidator {
        serde_json::from_slice::<TxnValidator>(data).unwrap()
    }
}
