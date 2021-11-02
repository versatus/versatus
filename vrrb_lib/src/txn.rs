use crate::pool::Pool;
use crate::state::NetworkState;
use crate::verifiable::Verifiable;
use crate::wallet::WalletAccount;
use bytebuffer::ByteBuffer;
use secp256k1::{Message, PublicKey, Secp256k1, Signature};
use serde::{Deserialize, Serialize};
use sha256::digest_bytes;
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Txn {
    pub txn_id: String,
    pub txn_timestamp: u128,
    pub sender_address: String,
    pub sender_public_key: String,
    pub receiver_address: String,
    pub txn_token: Option<String>,
    pub txn_amount: u128,
    pub txn_payload: String,
    pub txn_signature: String,
    pub validators: HashMap<String, bool>,
    pub nonce: u128,
}

impl Txn {
    pub fn new(
        sender: Arc<Mutex<WalletAccount>>,
        sender_address: String,
        receiver: String,
        amount: u128,
        nonce: u128,
    ) -> Txn {
        let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

        let payload = format!(
            "{},{},{},{},{},{}",
            &time.as_nanos().to_string(),
            &sender_address,
            &sender.lock().unwrap().pubkey.clone(),
            &receiver,
            &amount.to_string(),
            &nonce
        );
        let signature = sender.lock().unwrap().sign(&payload).unwrap();
        let uid_payload = format!(
            "{},{},{}",
            &payload,
            Uuid::new_v4().to_string(),
            &signature.to_string()
        );

        Txn {
            txn_id: digest_bytes(uid_payload.as_bytes()),
            txn_timestamp: time.as_nanos(),
            sender_address: sender_address,
            sender_public_key: sender.lock().unwrap().pubkey.clone(),
            receiver_address: receiver,
            txn_token: None,
            txn_amount: amount,
            txn_payload: payload,
            txn_signature: signature.to_string(),
            validators: HashMap::new(),
            nonce,
        }
    }

    // TODO: convert to_message into a function of the verifiable trait,
    // all verifiable objects need to be able to be converted to a message.
    pub fn to_string(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let as_string = serde_json::to_string(self).unwrap();
        as_string.as_bytes().iter().copied().collect()
    }

    pub fn from_bytes(data: &[u8]) -> Txn {
        serde_json::from_slice::<Txn>(data).unwrap()
    }

    pub fn from_string(string: &String) -> Txn {
        serde_json::from_str::<Txn>(string).unwrap()
    }

    pub fn get_field_names(&self) -> Vec<String> {
        vec![
            "txn_id".to_string(),
            "txn_timestamp".to_string(),
            "sender_address".to_string(),
            "sender_public_key".to_string(),
            "receiver_address".to_string(),
            "txn_token".to_string(),
            "txn_amount".to_string(),
            "txn_payload".to_string(),
            "txn_signature".to_string(),
            "txn_signature".to_string(),
            "validators".to_string(),
            "nonce".to_string(),
        ]
    }
}

impl Verifiable for Txn {
    fn verifiable(&self) -> bool {
        true
    }

    fn valid_txn(&self, network_state: &NetworkState, txn_pool: &Pool<String, Txn>) -> bool {
        if !self.valid_txn_signature() {
            return false;
        }

        if !self.valid_amount(network_state, txn_pool) {
            return false;
        }

        if !self.check_double_spend(txn_pool) {
            return false;
        }

        true
    }

    fn valid_txn_signature(&self) -> bool {
        let message = self.txn_payload.clone();
        let message_bytes = message.as_bytes().to_owned();

        let mut buffer = ByteBuffer::new();
        buffer.write_bytes(&message_bytes);
        while buffer.len() < 32 {
            buffer.write_u8(0);
        }
        let new_message = buffer.to_bytes();
        let message_hash = blake3::hash(&new_message);
        let message_hash = Message::from_slice(message_hash.as_bytes()).unwrap();
        let secp = Secp256k1::new();
        let valid = secp.verify(
            &message_hash,
            &Signature::from_str(&self.txn_signature).unwrap(),
            &PublicKey::from_str(&self.sender_public_key).unwrap(),
        );

        match valid {
            Ok(()) => return true,
            _ => return false,
        }
    }

    fn valid_amount(&self, network_state: &NetworkState, txn_pool: &Pool<String, Txn>) -> bool {
        let (_, pending_debits) = if let Some((credit_amount, debit_amount)) =
            network_state.pending_balance(self.sender_address.clone(), txn_pool)
        {
            (credit_amount, debit_amount)
        } else {
            (0, 0)
        };

        let mut address_balance = network_state.get_balance(&self.sender_address);

        address_balance = if let Some(amount) = address_balance.checked_sub(pending_debits) {
            amount
        } else {
            println!("Invalid balance, not enough coins!");
            return false;
        };

        if address_balance < self.txn_amount {
            println!("Invalid balance, not enough coins");
            return false;
        }
        true
    }

    fn check_double_spend(&self, txn_pool: &Pool<String, Txn>) -> bool {
        if let Some(txn) = txn_pool.pending.get(&self.txn_id) {
            if txn.txn_id == self.txn_id
                && (txn.txn_amount != self.txn_amount
                    || txn.receiver_address != self.receiver_address)
            {
                println!("Attempted double spend");
                return false;
            }
        };

        true
    }

    fn check_txn_nonce(&self, _network_state: &NetworkState) -> bool {
        //TODO: Add get_account_txn_nonce to network state to get the txn nonce
        true
    }
}

impl fmt::Display for Txn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Txn(\n \
            txn_id: {},\n \
            txn_timestamp: {},\n \
            sender_address: {},\n \
            sender_public_key: {},\n \
            receiver_address: {},\n \
            txn_token: {:?},\n \
            txn_amount: {},\n \
            txn_signature: {}",
            self.txn_id,
            self.txn_timestamp.to_string(),
            self.sender_address,
            self.sender_public_key,
            self.receiver_address,
            self.txn_token,
            self.txn_amount,
            self.txn_signature,
        )
    }
}
