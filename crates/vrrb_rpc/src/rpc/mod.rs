pub mod api;
pub mod client;
mod server;
mod server_impl;
use serde::{Deserialize, Serialize};
pub use server::*;
pub use server_impl::*;
use vrrb_core::txn::Token;

#[derive(Debug, Serialize, Deserialize)]
pub struct SignOpts {
    timestamp: i64,
    sender_address: String,
    sender_public_key: String,
    receiver_address: String,
    amount: u128,
    token: Token,
    nonce: u128,
    private_key: String,
}
