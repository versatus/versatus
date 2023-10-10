use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
};

use jsonrpsee::core::client::Client;
use primitives::Address;
use secp256k1::{ecdsa::Signature, Message, PublicKey, Secp256k1, SecretKey};
use serde::{Deserialize, Serialize};
use telemetry::error;
use thiserror::Error;
use vrrb_core::account::Account;
use vrrb_core::transactions::{Transaction, TransactionKind, Token, RpcTransactionDigest};
use vrrb_rpc::rpc::{
    api::{RpcApiClient, RpcTransactionRecord},
    client::create_client,
};

type WalletResult<Wallet> = Result<Wallet, WalletError>;

#[derive(Error, Debug)]
pub enum WalletError {
    #[error("RPC error: {0}")]
    RpcError(#[from] jsonrpsee::core::Error),

    #[error("API error: {0}")]
    ApiError(#[from] vrrb_rpc::ApiError),

    #[error("custom error")]
    Custom(String),
}

pub type AddressAlias = u32;

#[derive(Debug)]
pub struct Wallet {
    secret_key: SecretKey,
    welcome_message: String,
    client: Client,
    pub public_key: PublicKey,
    pub addresses: HashMap<AddressAlias, Address>,
    pub accounts: HashMap<Address, Account>,
    pub nonce: u128,
}

#[derive(Debug)]
pub struct WalletConfig {
    pub rpc_server_address: SocketAddr,
    pub secret_key: SecretKey,
    pub public_key: PublicKey,
    pub accounts: HashMap<Address, Account>,
    pub addresses: HashMap<AddressAlias, Address>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletInfo {
    pub secret_key: SecretKey,
    pub public_key: String,
    pub addresses: HashMap<u32, Address>,
    pub nonce: u128,
}

impl Default for WalletConfig {
    fn default() -> Self {
        let secp = Secp256k1::new();
        // NOTE: not meant to be used in production. Generate a random keypair from the
        // CLI
        let secret_key = SecretKey::from_slice(&[0xcd; 32]).unwrap();
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);
        let rpc_server_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9293);
        let accounts = HashMap::new();
        let addresses = HashMap::new();

        Self {
            rpc_server_address,
            secret_key,
            public_key,
            accounts,
            addresses,
        }
    }
}

impl Wallet {
    /// Initiates a new wallet.
    pub async fn new(config: WalletConfig) -> WalletResult<Self> {
        //TODO: Don't use random keypair, generate it from a
        //mnemonic phrase seed or read from file if already generated

        let secret_key = config.secret_key;
        let public_key = config.public_key;

        let addresses = config.addresses;
        let accounts = config.accounts;

        //TODO: get rpc server address from config file or env variable
        let client = create_client(config.rpc_server_address).await?;

        let welcome_message = format!(
            "{}\nSECRET KEY: {:?}\nPUBLIC KEY: {:?}\n",
            "DO NOT SHARE OR LOSE YOUR SECRET KEY:", &secret_key, &public_key,
        );

        let wallet = Wallet {
            secret_key,
            public_key,
            welcome_message,
            client,
            addresses,
            accounts,
            nonce: 0,
        };

        Ok(wallet)
    }

    pub fn info(&self) -> WalletInfo {
        WalletInfo {
            secret_key: self.secret_key,
            public_key: self.public_key.to_string(),
            addresses: self.addresses.clone(),
            nonce: self.nonce,
        }
    }

    pub async fn get_mempool(&self) -> Result<Vec<RpcTransactionRecord>, WalletError> {
        let mempool = self.client.get_full_mempool().await?;

        Ok(mempool)
    }

    pub async fn send_transaction(
        &mut self,
        address_number: u32,
        receiver: Address,
        amount: u128,
        token: Token,
        timestamp: i64,
    ) -> Result<RpcTransactionDigest, WalletError> {
        let addresses = self.addresses.clone();
        let sender_address = {
            if let Some(addr) = addresses.get(&address_number) {
                addr.clone()
            } else if let Some(addr) = addresses.get(&0) {
                addr.clone()
            } else {
                return Err(WalletError::Custom("wallet has no addresses".to_string()));
            }
        };

        let payload = utils::hash_data!(
            timestamp,
            sender_address.to_string(),
            self.public_key.to_string(),
            receiver.to_string(),
            amount,
            token,
            self.nonce.clone()
        );

        let signature = self.sign_transaction(&payload[..]);

        let transfer = TransactionKind::transfer_builder()
            .timestamp(timestamp)
            .sender_address(sender_address)
            .sender_public_key(self.public_key)
            .receiver_address(receiver)
            .token(token)
            .amount(amount)
            .signature(signature)
            .validators(HashMap::new())
            .nonce(self.nonce)
            .build_kind()
            .map_err(|_| WalletError::Custom("Failed to build transfer transaction".to_string()))?;

        self
            .client
            .create_txn(transfer.clone())
            .await
            .map_err(|err| {
                error!("{:?}", err.to_string());
                WalletError::Custom(format!("API Error: {}", err))
            })?;

        Ok(transfer.id().digest_string())
    }

    pub async fn get_transaction(
        &mut self,
        transaction_digest: RpcTransactionDigest,
    ) -> Option<RpcTransactionRecord> {
        let res = self.client.get_transaction(transaction_digest).await;

        if let Ok(value) = res {
            Some(value)
        } else {
            None
        }
    }

    pub async fn get_account(&mut self, address: Address) -> WalletResult<Account> {
        let account = self.client.get_account(address).await.map_err(|err| {
            error!("{:?}", err.to_string());

            WalletError::Custom(format!("API Error: {err}"))
        })?;

        Ok(account)
    }

    pub async fn list_transactions(
        &mut self,
        ids: Vec<RpcTransactionDigest>,
    ) -> HashMap<RpcTransactionDigest, RpcTransactionRecord> {
        let res = self.client.list_transactions(ids).await;

        if let Ok(values) = res {
            values
        } else {
            HashMap::new()
        }
    }

    fn sign_transaction(&mut self, payload: &[u8]) -> Signature {
        type H = secp256k1::hashes::sha256::Hash;
        let msg = Message::from_hashed_data::<H>(payload);
        self.secret_key.sign_ecdsa(msg)
    }

    pub fn get_welcome_message(&self) -> String {
        self.welcome_message.clone()
    }

    pub async fn restore_from_private_key(
        secret_key: String,
        rpc_server: SocketAddr,
    ) -> WalletResult<Self> {
        if let Ok(secretkey) = SecretKey::from_str(&secret_key) {
            let pubkey =
                vrrb_core::keypair::KeyPair::get_miner_public_key_from_secret_key(secretkey);

            let client = create_client(rpc_server).await?;

            let mut wallet = Wallet {
                secret_key: secretkey,
                welcome_message: String::new(),
                client,
                public_key: pubkey,
                addresses: HashMap::new(),
                accounts: HashMap::new(),
                nonce: 0,
            };

            wallet.get_new_address();

            let mut accounts = HashMap::new();
            let addresses = wallet.addresses.clone();
            for (_, addr) in addresses.iter() {
                let account = wallet.get_account(addr.clone()).await?;
                accounts.insert(addr.to_owned(), account);
            }

            wallet.accounts = accounts;

            let welcome_message = format!(
                "{}\nSECRET KEY: {:?}\nPUBLIC KEY: {:?}\nADDRESS: {}\n",
                "DO NOT SHARE OR LOSE YOUR SECRET KEY:",
                &wallet.secret_key,
                &wallet.public_key,
                &wallet.addresses.get(&1).unwrap(),
            );

            wallet.welcome_message = welcome_message;

            Ok(wallet)
        } else {
            Err(WalletError::Custom(
                "unable to restore wallet from secret key".to_string(),
            ))
        }
    }

    // Create an account for each address created
    pub fn get_new_address(&mut self) {
        let largest_address_index = self.addresses.len();
        let pk = self.public_key;
        let new_address = Address::new(pk);
        self.addresses
            .insert(largest_address_index as u32, new_address);
    }

    pub fn get_wallet_addresses(&self) -> HashMap<AddressAlias, Address> {
        self.addresses.clone()
    }

    pub async fn create_account(
        &mut self,
        alias: AddressAlias,
        public_key: PublicKey,
    ) -> Result<(Address, Account), WalletError> {
        let address = Address::new(public_key);
        let account = Account::new(address.clone());

        self.client
            .create_account(address.clone(), account.clone())
            .await
            .map_err(|err| WalletError::Custom(err.to_string()))?;

        self.addresses.insert(alias, address.clone());
        self.accounts.insert(address.clone(), account.clone());

        Ok((address, account))
    }
}
