pub mod api;
pub mod client;
mod server;
mod server_impl;
use serde::{Deserialize, Serialize};
pub use server::*;
pub use server_impl::*;
use vrrb_core::transactions::Token;

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq, Hash)]
pub struct SignOpts {
    pub timestamp: i64,
    pub sender_address: String,
    pub sender_public_key: String,
    pub receiver_address: String,
    pub amount: u128,
    pub token: Token,
    pub nonce: u128,
    pub private_key: String,
}
