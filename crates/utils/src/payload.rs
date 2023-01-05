#![allow(unused_imports)]
use secp256k1::Message;
use secp256k1::hashes::{sha256 as s256, Hash};
use sha256::digest;

#[macro_export]
macro_rules! create_payload {
    ($($x:expr),*) => {{
        let mut payload = String::new();

        $(
            payload.push_str(&format!("{:?}", $x));
        )*

        Message::from(s256::Hash::hash(&payload.as_bytes()))
    }};
}

#[macro_export]
macro_rules! hash_data {
    ($($x:expr),*) => {{
        
        let mut payload = String::new();

        $(
            payload.push_str(&format!("{:?}", $x));
        )*

        digest(payload.as_bytes())
    }};
}
