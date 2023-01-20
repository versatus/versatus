use std::fmt::Display;

use serde::{Deserialize, Serialize};
use sha2::Digest;

#[derive(Debug)]
pub enum TokenError {
    InsufficientFundsError,
}

impl Display for TokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Insufficient funds")
    }
}

impl std::error::Error for TokenError {}

#[derive(Debug, Default, Clone, Serialize, Deserialize, Eq, PartialEq, Digest)]
pub struct Token {
    pub contract_address: String,
    pub available_balance: i128,
    pub total_balance: i128,
}

impl Token {

    //used for adding a token to an account's addr
    //new addresses instantiated with VRRB token
    //TODO: figure out genesis configuration (VRRB token is an account)
    pub fn new_token(contract_address: String, available_balance: i128, total_balance: i128) -> Token {
        Token {
            contract_address,
            available_balance,
            total_balance,
        }
    }

    pub fn update_balance(&mut self, amount: i128) -> Result<(), TokenError> {
        if self.available_balance + amount < 0 {
            //add error handling
            TokenError::InsufficientFundsError;
        }
        self.available_balance + amount;
        //TODO: total balance needs to wait for txn confirmation
        self.total_balance + amount;

        Ok(())
    }
}

