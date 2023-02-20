use std::{collections::HashMap, net::SocketAddr, str::FromStr};

use jsonrpsee::core::client::Client;
use primitives::{digest::TransactionDigest, Address};
use secp256k1::{ecdsa::Signature, PublicKey, SecretKey};
use sha2::{Digest, Sha256};
use thiserror::Error;
use vrrb_core::{
    account::Account,
    keypair::{KeyPair, KeyPairError},
    txn::{TxToken, Txn},
};
use vrrb_rpc::rpc::{api::RpcClient, client::create_client};

type WalletResult<Wallet> = Result<Wallet, WalletError>;

#[derive(Error, Debug)]
pub enum WalletError {
    #[error("unable to create rpc client")]
    InvalidRpcClient(#[from] vrrb_rpc::ApiError),
    #[error("custom error")]
    Custom(String),
}

#[derive(Debug)]
pub struct Wallet {
    secret_key: SecretKey,
    welcome_message: String,
    client: Client,
    pub public_key: PublicKey,
    pub addresses: HashMap<u32, Address>,
    pub accounts: HashMap<Address, Account>,
    pub nonce: u128,
}

impl Wallet {
    /// Initiate a new wallet.
    pub async fn new(rpc_server: SocketAddr) -> WalletResult<Self> {
        //TODO: Don't use random keypair, generate it from a
        //mnemonic phrase seed
        let kp = KeyPair::random();

        //TODO: Change name of kp methods, not only miner secret key
        let sk = kp.get_miner_secret_key();
        let pk = kp.get_miner_public_key();

        let addresses = HashMap::new();
        let accounts = HashMap::new();
        //TODO: get rpc server address from config file
        let client = create_client(rpc_server).await?;

        let welcome_message = format!(
            "{}\nSECRET KEY: {:?}\nPUBLIC KEY: {:?}\n",
            "DO NOT SHARE OR LOSE YOUR SECRET KEY:", &sk, &pk,
        );

        let mut wallet = Wallet {
            secret_key: sk.clone(),
            welcome_message,
            client,
            public_key: pk.clone(),
            addresses,
            accounts,
            nonce: 0,
        };

        let res = wallet.create_account().await;
        #[cfg(debug_assertions)]
        println!("{:?}", res);

        if let Ok((address, account)) = res {
            #[cfg(debug_assertions)]
            println!("{:?}", address);
            #[cfg(debug_assertions)]
            println!("{:?}", account);
            wallet.addresses.insert(0, address.clone());
            wallet.accounts.insert(address.clone(), account);
            #[cfg(debug_assertions)]
            println!("{:?}", wallet.addresses);
            #[cfg(debug_assertions)]
            println!("{:?}", wallet.accounts);
            let welcome_message = format!(
                "{}\nSECRET KEY: {:?}\nPUBLIC KEY: {:?}\nADDRESS: {}\n",
                "DO NOT SHARE OR LOSE YOUR SECRET KEY:", &sk, &pk, &address,
            );
            wallet.welcome_message = welcome_message;
        }

        Ok(wallet)
    }

    pub async fn send_txn(
        &mut self,
        address_number: u32,
        receiver: String,
        amount: u128,
        token: Option<TxToken>,
    ) -> Result<TransactionDigest, WalletError> {
        let time = chrono::Utc::now().timestamp();

        let addresses = self.addresses.clone();
        let sender_address = {
            if let Some(addr) = addresses.get(&address_number) {
                addr
            } else {
                if let Some(addr) = addresses.get(&0) {
                    addr
                } else {
                    return Err(WalletError::Custom("wallet has no addresses".to_string()));
                }
            }
        };

        let payload = format!(
            "{},{},{},{},{},{:?},{}",
            &time,
            &sender_address,
            &hex::encode(self.public_key.to_string().as_bytes().to_vec().clone()),
            &receiver,
            &amount,
            &token,
            &self.nonce.clone()
        );

        let mut hasher = Sha256::new();
        hasher.update(payload.as_bytes());
        let payload_hash = hasher.finalize();

        let signature = self
            .sign_txn(&payload_hash[..])
            .map_err(|err| WalletError::Custom(err.to_string()))?;

        let txn = Txn::new(vrrb_core::txn::NewTxnArgs {
            sender_address: sender_address.to_string(),
            sender_public_key: self.public_key.to_string().as_bytes().to_vec(),
            receiver_address: receiver,
            token,
            amount,
            payload: Some(payload),
            signature: signature.to_string().as_bytes().to_vec(),
            validators: Some(HashMap::new()),
            nonce: self.nonce,
        });

        let _ = self.client.create_txn(txn.clone()).await;

        let txn_string = serde_json::to_string(&txn)
            .map_err(|_| WalletError::Custom("unable to convert txn to string".to_string()));

        let return_res = {
            if let Ok(txn_string) = txn_string {
                let mut hasher = Sha256::new();
                hasher.update(txn_string.as_bytes());
                let txn_hash = hasher.finalize();

                let digest = TransactionDigest::from(&txn_hash[..]);

                Ok(digest)
            } else {
                Err(WalletError::Custom(
                    "unable to create txn digest".to_string(),
                ))
            }
        };

        return_res
    }

    pub async fn get_transaction(&mut self, transaction_digest: TransactionDigest) -> Option<Txn> {
        let res = self.client.get_transaction(transaction_digest).await;

        if let Ok(value) = res {
            return Some(value);
        } else {
            return None;
        }
    }

    pub async fn get_account(&mut self, address: Address) -> Option<Account> {
        let res = self.client.get_account(address).await;

        if let Ok(value) = res {
            return Some(value);
        } else {
            return None;
        }
    }

    pub async fn list_transactions(
        &mut self,
        digests: Vec<TransactionDigest>,
    ) -> HashMap<TransactionDigest, Txn> {
        let res = self.client.list_transactions(digests).await;

        if let Ok(values) = res {
            return values;
        } else {
            return HashMap::new();
        }
    }

    fn sign_txn(&mut self, payload: &[u8]) -> Result<Signature, KeyPairError> {
        KeyPair::ecdsa_signature(payload, &self.secret_key.secret_bytes().to_vec())
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
                let ret = wallet.get_account(addr.clone()).await;
                if let Some(account) = ret {
                    accounts.insert(addr.to_owned(), account);
                }
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
            return Err(WalletError::Custom(
                "unable to restore wallet from secret key".to_string(),
            ));
        }
    }

    // Create an account for each address created
    pub fn get_new_address(&mut self) {
        let largest_address_index = self.addresses.len();
        let pk = self.public_key.clone();
        let new_address = Address::new(pk);
        self.addresses
            .insert(largest_address_index as u32, new_address);
    }

    pub fn get_wallet_addresses(&self) -> HashMap<u32, Address> {
        self.addresses.clone()
    }

    pub async fn create_account(&mut self) -> Result<(Address, Account), WalletError> {
        let pk = self.public_key.clone();
        let account = Account::new(pk.clone());
        let address = Address::new(pk.clone());
        let _ = self
            .client
            .create_account(address.clone(), account.clone())
            .await
            .map_err(|err| WalletError::Custom(err.to_string()));

        Ok((address, account))
    }
}
