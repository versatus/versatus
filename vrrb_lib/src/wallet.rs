use crate::block::Block;
use crate::claim::Claim;
use crate::state::NetworkState;
use crate::txn::Txn;
use bytebuffer::ByteBuffer;
use ritelinked::LinkedHashMap;
use secp256k1::Error;
use secp256k1::{
    key::{PublicKey, SecretKey},
    Signature,
};
use secp256k1::{Message, Secp256k1};
use serde::{Deserialize, Serialize};
use sha256::digest_bytes;
use std::fmt;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

const STARTING_BALANCE: u128 = 1000;

/// The WalletAccount struct is the user/node wallet in which coins, tokens and contracts
/// are held. The WalletAccount has a private/public keypair
/// phrase are used to restore the Wallet. The private key is
/// also used to sign transactions, claims and mined blocks for network validation.
/// Private key signatures can be verified with the wallet's public key, the message that was
/// signed and the signature.
#[derive(Debug, Serialize, Deserialize)]
pub struct WalletAccount {
    secretkey: String,
    welcome_message: String,
    pub pubkey: String,
    pub addresses: LinkedHashMap<u32, String>,
    pub total_balances: LinkedHashMap<String, LinkedHashMap<String, u128>>,
    pub available_balances: LinkedHashMap<String, LinkedHashMap<String, u128>>,
    pub claims: LinkedHashMap<u128, Claim>,
    pub txn_nonce: u128,
}

impl WalletAccount {
    /// Initiate a new wallet.
    pub fn new() -> WalletAccount {
        // Initialize a new Secp256k1 context
        let secp = Secp256k1::new();

        // Generate a random number used to seed the new keypair for the wallet
        // TODO: Instead of using the rng, use a mnemonic seed.
        let mut rng = rand::thread_rng();
        // Generate a new secret/public key pair using the random seed.
        let (secret_key, public_key) = secp.generate_keypair(&mut rng);
        // Generate 100 addresses by hashing a universally unique IDs + secret_key + public_key
        let mut address_bytes = public_key.to_string().as_bytes().to_vec();
        address_bytes.push(1u8);
        let address = digest_bytes(digest_bytes(&address_bytes).as_bytes());
        // add the testnet prefix to the wallet address (TODO: add handling of testnet/mainnet)
        let mut address_prefix: String = "0x192".to_string();
        // push the hashed uuid string to the end of the address prefix
        address_prefix.push_str(&address);

        // Print the private key string so that the user can save it.
        // TODO: require a confirmation the private key being saved by the user
        let welcome_message = format!(
            "{}\nSECRET KEY: {:?}\nPUBLIC KEY: {:?}\nADDRESS: {}\n",
            "DO NOT SHARE OR LOSE YOUR SECRET KEY:",
            &secret_key,
            &public_key,
            &address_prefix,
        );
        let mut addresses = LinkedHashMap::new();
        addresses.insert(1, address_prefix.clone());

        let mut total_balances = LinkedHashMap::new();
        let mut vrrb_balances = LinkedHashMap::new();
        vrrb_balances.insert("VRRB".to_string(), STARTING_BALANCE);
        total_balances.insert(address_prefix.clone(), vrrb_balances);

        // Generate a wallet struct by assigning the variables to the fields.
        let wallet = Self {
            secretkey: secret_key.to_string(),
            welcome_message,
            pubkey: public_key.to_string(),
            addresses,
            total_balances: total_balances.clone(),
            available_balances: total_balances,
            claims: LinkedHashMap::new(),
            txn_nonce: 0,
        };

        wallet
    }

    pub fn get_welcome_message(&self) -> String {
        self.welcome_message.clone()
    }

    pub fn restore_from_private_key(private_key: String) -> WalletAccount {
        let secretkey = SecretKey::from_str(&private_key).unwrap();
        let secp = Secp256k1::new();
        let pubkey = PublicKey::from_secret_key(&secp, &secretkey);


        let mut wallet = WalletAccount {
            secretkey: secretkey.to_string(),
            welcome_message: String::new(),
            pubkey: pubkey.to_string(),
            addresses: LinkedHashMap::new(),
            total_balances: LinkedHashMap::new(),
            available_balances: LinkedHashMap::new(),
            claims: LinkedHashMap::new(),
            txn_nonce: 0,
        };

        wallet.get_new_addresses(1);

        let welcome_message = format!(
            "{}\nSECRET KEY: {:?}\nPUBLIC KEY: {:?}\nADDRESS: {}\n",
            "DO NOT SHARE OR LOSE YOUR SECRET KEY:",
            &wallet.secretkey,
            &wallet.pubkey,
            &wallet.addresses.get(&1).unwrap(),
        );

        wallet.welcome_message = welcome_message;

        wallet
    }

    pub fn get_txn_nonce(&mut self, _network_state: &NetworkState) {
        // TODO: add a get_account_txn_nonce() function to network state to update
        // txn nonce in walet when restored.
    }

    pub fn get_new_addresses(&mut self, number_of_addresses: u8) {
        let mut counter = 1u8;
        (counter..=number_of_addresses).for_each(|n| {
            let mut address_bytes = self.pubkey.as_bytes().to_vec();
            address_bytes.push(n);
            let address = digest_bytes(digest_bytes(&address_bytes).as_bytes());
            let mut address_prefix: String = "0x192".to_string();
            address_prefix.push_str(&address);
            self.addresses.insert(n as u32, address_prefix);
            counter += 1
        })
    }

    pub fn get_wallet_addresses(&self) -> LinkedHashMap<u32, String> {
        self.addresses.clone()
    }

    pub fn render_balances(&self) -> LinkedHashMap<String, LinkedHashMap<String, u128>> {
        self.total_balances.clone()
    }

    pub fn update_balances(&mut self, network_state: NetworkState) {
        let mut balance_map = LinkedHashMap::new();
        self.get_balances(network_state)
            .iter()
            .for_each(|(address, balance)| {
                let mut vrrb_map = LinkedHashMap::new();
                vrrb_map.insert("VRRB".to_string(), *balance);
                balance_map.insert(address.clone(), vrrb_map);
            });

        self.total_balances = balance_map;
    }

    pub fn get_balances(&self, network_state: NetworkState) -> LinkedHashMap<String, u128> {
        let mut balance_map = LinkedHashMap::new();

        self.addresses.iter().for_each(|(_, address)| {
            let balance = network_state.get_balance(&address);
            balance_map.insert(address.clone(), balance);
        });

        balance_map
    }

    pub fn get_address_balance(
        &mut self,
        network_state: NetworkState,
        address_number: u32,
    ) -> Option<u128> {
        self.update_balances(network_state);
        if let Some(address) = self.addresses.get(&address_number) {
            if let Some(entry) = self.total_balances.get(&address.clone()) {
                if let Some(amount) = entry.get("VRRB") {
                    return Some(*amount);
                } else {
                    return None;
                }
            } else {
                return None;
            }
        } else {
            return None;
        }
    }

    pub fn n_claims_owned(&self) -> u128 {
        self.claims.len() as u128
    }

    pub fn get_claims(&self) -> LinkedHashMap<u128, Claim> {
        self.claims.clone()
    }

    pub fn get_pubkey(&self) -> String {
        self.pubkey.clone()
    }

    pub fn get_secretkey(&self) -> String {
        self.secretkey.clone()
    }

    pub fn sign(&self, message: &str) -> Result<Signature, Error> {
        let message_bytes = message.as_bytes().to_owned();
        let mut buffer = ByteBuffer::new();
        buffer.write_bytes(&message_bytes);
        while buffer.len() < 32 {
            buffer.write_u8(0);
        }

        let new_message = buffer.to_bytes();
        let message_hash = blake3::hash(&new_message);
        let message_hash = Message::from_slice(message_hash.as_bytes())?;
        let secp = Secp256k1::new();
        let sk = SecretKey::from_str(&self.secretkey).unwrap();
        let sig = secp.sign(&message_hash, &sk);
        Ok(sig)
    }

    /// Verify a signature with the signers public key, the message payload and the signature.
    pub fn verify(message: String, signature: Signature, pk: PublicKey) -> Result<bool, Error> {
        let message_bytes = message.as_bytes().to_owned();

        let mut buffer = ByteBuffer::new();
        buffer.write_bytes(&message_bytes);
        while buffer.len() < 32 {
            buffer.write_u8(0);
        }
        let new_message = buffer.to_bytes();
        let message_hash = blake3::hash(&new_message);
        let message_hash = Message::from_slice(message_hash.as_bytes())?;
        let secp = Secp256k1::new();
        let valid = secp.verify(&message_hash, &signature, &pk);

        match valid {
            Ok(()) => Ok(true),
            _ => Err(Error::IncorrectSignature),
        }
    }

    pub fn txns_in_block(&mut self, block: &Block) {
        let _my_txns = {
            let mut some_txn = false;
            self.addresses.iter().for_each(|(_, address)| {
                let mut cloned_data = block.txns.clone();
                cloned_data.retain(|_, txn| {
                    txn.receiver_address == address.clone() || txn.sender_address == address.clone()
                });
                if !cloned_data.is_empty() {
                    some_txn = true;
                }
            });
            some_txn
        };
    }

    pub fn send_txn(
        &mut self,
        address_number: u32,
        receiver: String,
        amount: u128,
    ) -> Result<Txn, Error> {
        let txn = Txn::new(
            Arc::new(Mutex::new(self.clone())),
            self.addresses.get(&address_number).unwrap().clone(),
            receiver,
            amount,
            self.txn_nonce,
        );
        self.txn_nonce += 1;

        Ok(txn)
    }

    pub fn get_address(&mut self, address_number: u32) -> String {
        if let Some(address) = self.addresses.get(&address_number) {
            return address.to_string();
        } else {
            while self.addresses.len() < address_number as usize {
                self.generate_new_address()
            }
            self.get_address(address_number)
        }
    }

    pub fn generate_new_address(&mut self) {
        let uid = Uuid::new_v4().to_string();
        let address_number: u32 = self.addresses.len() as u32 + 1u32;
        let payload = format!("{},{},{}", &address_number, &uid, &self.pubkey);
        let address = digest_bytes(payload.as_bytes());
        self.addresses.insert(address_number, address);
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let as_string = serde_json::to_string(self).unwrap();
        as_string.as_bytes().iter().copied().collect()
    }

    pub fn from_bytes(data: &[u8]) -> WalletAccount {
        let mut buffer: Vec<u8> = vec![];
        data.iter().for_each(|x| buffer.push(*x));
        let to_string = String::from_utf8(buffer).unwrap();
        serde_json::from_str::<WalletAccount>(&to_string).unwrap()
    }
}

impl fmt::Display for WalletAccount {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Wallet(\n \
            address: {:?},\n \
            balances: {:?},\n \
            available_balance: {:?},\n \
            claims_owned: {}",
            self.addresses,
            self.total_balances,
            self.available_balances,
            self.claims.len()
        )
    }
}

impl Clone for WalletAccount {
    fn clone(&self) -> WalletAccount {
        WalletAccount {
            secretkey: self.secretkey.clone(),
            welcome_message: self.welcome_message.clone(),
            pubkey: self.pubkey.clone(),
            addresses: self.addresses.clone(),
            total_balances: self.total_balances.clone(),
            available_balances: self.available_balances.clone(),
            claims: self.claims.clone(),
            txn_nonce: self.txn_nonce.clone(),
        }
    }
}
