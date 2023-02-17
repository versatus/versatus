// //FEATURE TAG(S): Block, Chain & Syncing, Rewards, Develop SDK, Develop API
// for // Distributed Programs, Remote Procedure Calls.
// use std::error::Error;
// /// The wallet module contains very basic Wallet type and methods related to
// it. /// This will largely be replaced under the proposed protocol, however,
// for the /// prototype this version served its purpose
// use std::{
//     collections::HashMap,
//     fmt,
//     str::FromStr,
//     time::{SystemTime, UNIX_EPOCH},
// };
//
// use bytebuffer::ByteBuffer;
// use ritelinked::LinkedHashMap;
// use serde::{Deserialize, Serialize};
// use sha256::digest;
// use state::state::NetworkState;
// use uuid::Uuid;
// use vrrb_core::{
//     accountable::Accountable,
//     claim::Claim,
//     keypair::{KeyPair, MinerSk as SecretKey},
//     txn::Txn,
// };
//
// const STARTING_BALANCE: u128 = 1000;
//
// /// The WalletAccount struct is the user/node wallet in which coins, tokens
// and /// contracts are held. The WalletAccount has a private/public keypair
// /// phrase are used to restore the Wallet. The private key is
// /// also used to sign transactions, claims and mined blocks for network
// /// validation. Private key signatures can be verified with the wallet's
// public /// key, the message that was signed and the signature.
// #[derive(Debug, Serialize, Deserialize)]
// pub struct WalletAccount {
//     secret_key: Vec<u8>,
//     welcome_message: String,
//     pub public_key: Vec<u8>,
//     pub addresses: LinkedHashMap<u32, String>,
//     pub total_balances: LinkedHashMap<String, LinkedHashMap<String, u128>>,
//     pub available_balances: LinkedHashMap<String, LinkedHashMap<String,
// u128>>,     pub claims: LinkedHashMap<u128, Claim>,
//     pub nonce: u128,
// }
//
// impl Default for WalletAccount {
//     fn default() -> Self {
//         let kp = KeyPair::random();
//         let secret_key = kp.get_miner_secret_key();
//         let public_key = kp.get_miner_public_key();
//         // Generate 100 addresses by hashing a universally unique IDs +
// secret_key +         // public_key
//         let mut address_bytes = public_key.to_string().as_bytes().to_vec();
//         address_bytes.push(1u8);
//         // TODO: Is double hashing neccesary?
//         let address = digest(digest(&*address_bytes).as_bytes());
//         // add the testnet prefix to the wallet address (TODO: add handling
// of         // testnet/mainnet)
//         let mut address_prefix: String = "0x192".to_string();
//         // push the hashed uuid string to the end of the address prefix
//         address_prefix.push_str(&address);
//
//         // Print the private key string so that the user can save it.
//         // TODO: require a confirmation the private key being saved by the
// user         let welcome_message = format!(
//             "{}\nSECRET KEY: {:?}\nPUBLIC KEY: {:?}\nADDRESS: {}\n",
//             "DO NOT SHARE OR LOSE YOUR SECRET KEY:", &secret_key,
// &public_key, &address_prefix,         );
//
//         let mut addresses = LinkedHashMap::new();
//         addresses.insert(1, address_prefix.clone());
//
//         let mut total_balances = LinkedHashMap::new();
//         let mut vrrb_balances = LinkedHashMap::new();
//         vrrb_balances.insert("VRRB".to_string(), STARTING_BALANCE);
//         total_balances.insert(address_prefix.clone(), vrrb_balances);
//
//         // Generate a wallet struct by assigning the variables to the fields.
//         Self {
//             secret_key: secret_key.secret_bytes().to_vec(),
//             welcome_message,
//             public_key: public_key.serialize().to_vec(),
//             addresses,
//             total_balances: total_balances.clone(),
//             available_balances: total_balances,
//             claims: LinkedHashMap::new(),
//             nonce: 0,
//         }
//     }
// }
//
// impl WalletAccount {
//     /// Initiate a new wallet.
//     pub fn new() -> Self {
//         Self::default()
//     }
//
//     pub fn get_welcome_message(&self) -> String {
//         self.welcome_message.clone()
//     }
//
//     pub fn restore_from_private_key(private_key: String) -> Self {
//         let secretkey = SecretKey::from_str(&private_key).unwrap();
//         let pubkey =
// vrrb_core::keypair::KeyPair::get_miner_public_key_from_secret_key(secretkey);
//         let mut wallet = WalletAccount {
//             secret_key: secretkey.secret_bytes().to_vec(),
//             welcome_message: String::new(),
//             public_key: pubkey.serialize().to_vec(),
//             addresses: LinkedHashMap::new(),
//             total_balances: LinkedHashMap::new(),
//             available_balances: LinkedHashMap::new(),
//             claims: LinkedHashMap::new(),
//             nonce: 0,
//         };
//
//         wallet.get_new_addresses(1);
//
//         let welcome_message = format!(
//             "{}\nSECRET KEY: {:?}\nPUBLIC KEY: {:?}\nADDRESS: {}\n",
//             "DO NOT SHARE OR LOSE YOUR SECRET KEY:",
//             &wallet.secret_key,
//             &wallet.public_key,
//             &wallet.addresses.get(&1).unwrap(),
//         );
//
//         wallet.welcome_message = welcome_message;
//
//         wallet
//     }
//
//     pub fn get_txn_nonce(&mut self, _network_state: &NetworkState) {
//         // TODO: add a get_account_txn_nonce() function to network state to
//         // update txn nonce in walet when restored.
//     }
//
//     pub fn get_new_addresses(&mut self, number_of_addresses: u8) {
//         let mut counter = 1u8;
//         (counter..=number_of_addresses).for_each(|n| {
//             let mut address_bytes = self.public_key.clone();
//             address_bytes.push(n);
//             // TODO: Is double hashing neccesary?
//             let address = digest(digest(&*address_bytes).as_bytes());
//             let mut address_prefix: String = "0x192".to_string();
//             address_prefix.push_str(&address);
//             self.addresses.insert(n as u32, address_prefix);
//             counter += 1
//         })
//     }
//
//     pub fn get_wallet_addresses(&self) -> LinkedHashMap<u32, String> {
//         self.addresses.clone()
//     }
//
//     pub fn render_balances(&self) -> LinkedHashMap<String,
// LinkedHashMap<String, u128>> {         self.total_balances.clone()
//     }
//
//     pub fn update_balances(&mut self, network_state: NetworkState) {
//         let mut balance_map = LinkedHashMap::new();
//         self.get_balances(network_state)
//             .iter()
//             .for_each(|(address, balance)| {
//                 let mut vrrb_map = LinkedHashMap::new();
//                 vrrb_map.insert("VRRB".to_string(), *balance);
//                 balance_map.insert(address.clone(), vrrb_map);
//             });
//
//         self.total_balances = balance_map;
//     }
//
//     pub fn get_balances(&self, network_state: NetworkState) ->
// LinkedHashMap<String, u128> {         let mut balance_map =
// LinkedHashMap::new();
//
//         self.addresses.iter().for_each(|(_, address)| {
//             let balance = network_state.get_balance(address);
//             balance_map.insert(address.clone(), balance);
//         });
//
//         balance_map
//     }
//
//     pub fn get_address_balance(
//         &mut self,
//         network_state: NetworkState,
//         address_number: u32,
//     ) -> Option<u128> {
//         self.update_balances(network_state);
//         if let Some(address) = self.addresses.get(&address_number) {
//             if let Some(entry) = self.total_balances.get(&address.clone()) {
//                 entry.get("VRRB").copied()
//             } else {
//                 None
//             }
//         } else {
//             None
//         }
//     }
//
//     pub fn n_claims_owned(&self) -> u128 {
//         self.claims.len() as u128
//     }
//
//     pub fn get_claims(&self) -> LinkedHashMap<u128, Claim> {
//         self.claims.clone()
//     }
//
//     /// Checks if the local wallet has any transactions in the most recent
// block     pub fn txns_in_block(&mut self, txns: &LinkedHashMap<String, Txn>)
// {         let _my_txns = {
//             let mut some_txn = false;
//             self.addresses.iter().for_each(|(_, address)| {
//                 let mut cloned_data = txns.clone();
//                 cloned_data.retain(|_, txn| {
//                     true
//
//                     // TODO: re-enable this
//                     // txn.receivable() == address.clone() || txn.payable()
// ==                     // Some(address.clone())
//                 });
//
//                 if !cloned_data.is_empty() {
//                     some_txn = true;
//                 }
//             });
//             some_txn
//         };
//     }
//
//     /// Structures a `Txn` and returns a Result enum with either Ok(Txn) or
// an     /// Error if the local wallet cannot create a Txn for whatever reason
//     pub fn send_txn(
//         &mut self,
//         address_number: u32,
//         receiver: String,
//         amount: u128,
//     ) -> Result<Txn, Box<dyn std::error::Error>> {
//         let time = SystemTime::now()
//             .duration_since(UNIX_EPOCH)
//             .unwrap()
//             .as_nanos();
//         let sender_address = {
//             if let Some(addr) = self.addresses.get(&address_number) {
//                 addr
//             } else {
//                 let n_new_addresses = address_number as usize -
// self.addresses.len();                 self.get_new_addresses(n_new_addresses
// as u8);                 self.addresses.get(&address_number).unwrap()
//             }
//         };
//
//         let payload = format!(
//             "{},{},{},{},{},{}",
//             &time,
//             &sender_address,
//             &hex::encode(self.public_key.clone()),
//             &receiver,
//             &amount,
//             &self.nonce
//         );
//
//         let signature = KeyPair::ecdsa_signature(payload.as_bytes(),
// &self.secret_key)?             .to_string()
//             .as_bytes()
//             .to_vec();
//
//         let txn = Txn::new(vrrb_core::txn::NewTxnArgs {
//             sender_address: sender_address.to_string(),
//             sender_public_key: self.public_key.clone(),
//             receiver_address: receiver,
//             token: None,
//             amount,
//             payload: Some(payload),
//             signature,
//             validators: Some(HashMap::new()),
//             nonce: self.nonce,
//         });
//
//         Ok(txn)
//     }
//
//     /// Gets the local address of a wallet given an address number (naive HD
//     /// wallet)
//     pub fn get_address(&mut self, address_number: u32) -> String {
//         if let Some(address) = self.addresses.get(&address_number) {
//             address.to_string()
//         } else {
//             while self.addresses.len() < address_number as usize {
//                 self.generate_new_address()
//             }
//             self.get_address(address_number)
//         }
//     }
//
//     /// Generates a new address for the wallet based on the public key and a
//     /// unique ID
//     pub fn generate_new_address(&mut self) {
//         let uid = Uuid::new_v4().to_string();
//         let address_number: u32 = self.addresses.len() as u32 + 1u32;
//         let payload = format!(
//             "{},{},{}",
//             &address_number,
//             &uid,
//             &hex::encode(self.public_key.clone())
//         );
//         let address = digest(payload.as_bytes());
//         self.addresses.insert(address_number, address);
//     }
//
//     /// Serializes the wallet into a vector of bytes.
//     pub fn as_bytes(&self) -> Vec<u8> {
//         let as_string = serde_json::to_string(self).unwrap();
//         as_string.as_bytes().to_vec()
//     }
//
//     /// Deserializes a wallet from a byte array
//     pub fn from_bytes(data: &[u8]) -> WalletAccount {
//         let mut buffer: Vec<u8> = vec![];
//         data.iter().for_each(|x| buffer.push(*x));
//         let to_string = String::from_utf8(buffer).unwrap();
//         serde_json::from_str::<WalletAccount>(&to_string).unwrap()
//     }
// }
//
// impl fmt::Display for WalletAccount {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(
//             f,
//             "Wallet(\n \
//             address: {:?},\n \
//             balances: {:?},\n \
//             available_balance: {:?},\n \
//             claims_owned: {}",
//             self.addresses,
//             self.total_balances,
//             self.available_balances,
//             self.claims.len()
//         )
//     }
// }
//
// impl Clone for WalletAccount {
//     fn clone(&self) -> WalletAccount {
//         WalletAccount {
//             secret_key: self.secret_key.clone(),
//             welcome_message: self.welcome_message.clone(),
//             public_key: self.public_key.clone(),
//             addresses: self.addresses.clone(),
//             total_balances: self.total_balances.clone(),
//             available_balances: self.available_balances.clone(),
//             claims: self.claims.clone(),
//             nonce: self.nonce,
//         }
//     }
// }
