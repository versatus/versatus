use primitives::{Address, PublicKey, SecretKey};
use ritelinked::LinkedHashMap;
use secp256k1::{Message, Secp256k1};
use vrrb_core::{
    account::Account,
    keypair::Keypair,
    transactions::{
        generate_transfer_digest_vec, NewTransferArgs, Transaction, TransactionDigest,
        TransactionKind, Transfer,
    },
};

pub const N_ALPHANET_RECEIVERS: usize = 4;

// 50% after one year, then monthly for 12 months
const CONTRIBUTOR_VESTING: VestingConfig = VestingConfig {
    cliff_fraction: 0.5f64,
    cliff_years: 1f64,
    unlocks: 12,
    unlock_years: 1f64,
};

// 25% after half year, then monthly for 18  months
const INVESTOR_VESTING: VestingConfig = VestingConfig {
    cliff_fraction: 0.25f64,
    cliff_years: 0.75f64,
    unlocks: 18,
    unlock_years: 1.5f64,
};

#[derive(Debug, Clone)]
pub struct VestingConfig {
    pub cliff_fraction: f64,
    pub cliff_years: f64,
    pub unlocks: usize,
    pub unlock_years: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GenesisReceiverKind {
    Investor,
    Contributor,
}

#[derive(Debug, Clone)]
pub struct GenesisReceiver {
    pub address: Address,
    pub genesis_receiver_kind: GenesisReceiverKind,
    pub vesting_config: Option<VestingConfig>,
}

impl GenesisReceiver {
    fn new(
        address: Address,
        genesis_receiver_kind: GenesisReceiverKind,
        vesting_config: VestingConfig,
    ) -> Self {
        Self {
            address,
            genesis_receiver_kind,
            vesting_config: Some(vesting_config),
        }
    }
    #[cfg(test)]
    fn is_contributor(&self) -> bool {
        self.genesis_receiver_kind == GenesisReceiverKind::Contributor
    }
    #[cfg(test)]
    fn is_investor(&self) -> bool {
        self.genesis_receiver_kind == GenesisReceiverKind::Investor
    }
}

#[derive(Debug, Clone)]
pub struct GenesisConfig {
    pub sender: Address,
    pub receivers: Vec<GenesisReceiver>,
}

impl GenesisConfig {
    pub fn new(sender: Address) -> Self {
        Self {
            sender,
            receivers: Vec::new(),
        }
    }
}

pub fn create_vesting(
    sender_keypair: Keypair,
    sender_address: Address,
    genesis_receiver: &GenesisReceiver,
) -> (TransactionDigest, TransactionKind) {
    let tx_kind = create_txn_from_addresses(
        sender_keypair,
        sender_address,
        genesis_receiver.address.clone(),
        vec![],
    );
    (tx_kind.id(), tx_kind)
}

pub fn generate_genesis_txns(
    n: usize,
    sender_keypair: Keypair,
    genesis_config: &mut GenesisConfig,
) -> LinkedHashMap<TransactionDigest, TransactionKind> {
    let mut genesis_txns: LinkedHashMap<TransactionDigest, TransactionKind> =
        LinkedHashMap::with_capacity(n);

    if n != 0 {
        let mut receivers = Vec::with_capacity(n);
        let receiver_keysets = create_genesis_keysets(n);
        let lower_half = |n: usize| 0..(n / 2);
        let upper_half = |n: usize| (n / 2)..;
        let contributor_keysets = &receiver_keysets[lower_half(n)];
        let investor_keysets = &receiver_keysets[upper_half(n)];

        for (_, public_key) in contributor_keysets {
            let contributor = GenesisReceiver::new(
                Address::new(*public_key),
                GenesisReceiverKind::Contributor,
                CONTRIBUTOR_VESTING,
            );
            let vesting_txn = create_vesting(
                sender_keypair.clone(),
                genesis_config.sender.clone(),
                &contributor,
            );
            genesis_txns.insert(vesting_txn.0, vesting_txn.1);
            receivers.push(contributor);
        }
        for (_, public_key) in investor_keysets {
            let investor = GenesisReceiver::new(
                Address::new(*public_key),
                GenesisReceiverKind::Investor,
                INVESTOR_VESTING,
            );
            let vesting_txn = create_vesting(
                sender_keypair.clone(),
                genesis_config.sender.clone(),
                &investor,
            );
            genesis_txns.insert(vesting_txn.0, vesting_txn.1);
            receivers.push(investor);
        }

        genesis_config.receivers = receivers;
    }
    genesis_txns
}

fn create_genesis_keyset(m: usize) -> (SecretKey, PublicKey) {
    type H = secp256k1::hashes::sha256::Hash;
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_hashed_data::<H>(format!("genesis_member-{m}").as_bytes());
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);

    (secret_key, public_key)
}

fn create_genesis_keysets(n: usize) -> Vec<(SecretKey, PublicKey)> {
    (0..n).map(create_genesis_keyset).collect()
}

fn create_txn_from_addresses(
    sender_keypair: Keypair,
    sender: Address,
    receiver: Address,
    validators: Vec<(String, bool)>,
) -> TransactionKind {
    let sk = sender_keypair.get_miner_secret_key();
    let pk = sender_keypair.get_miner_public_key();
    let amount = 100u128.pow(2);
    let sender_acct = Account::new(sender.clone());
    let validators = validators
        .iter()
        .map(|(k, v)| (k.to_string(), *v))
        .collect();

    let txn_args = NewTransferArgs {
        timestamp: chrono::Utc::now().timestamp(),
        sender_address: sender,
        sender_public_key: *pk,
        receiver_address: receiver,
        token: None,
        amount,
        signature: sk
            .sign_ecdsa(Message::from_hashed_data::<secp256k1::hashes::sha256::Hash>(b"vrrb")),
        validators: Some(validators),
        nonce: sender_acct.nonce() + 1,
    };

    let mut txn = TransactionKind::Transfer(Transfer::new(txn_args));

    txn.sign(sk);

    let txn_digest_vec = generate_transfer_digest_vec(
        txn.timestamp(),
        txn.sender_address().to_string(),
        txn.sender_public_key(),
        txn.receiver_address().to_string(),
        txn.token().clone(),
        txn.amount(),
        txn.nonce(),
    );

    let _digest = TransactionDigest::from(txn_digest_vec);

    txn
}

#[cfg(test)]
mod tests {
    use std::assert_eq;

    use crate::{generate_genesis_txns, GenesisConfig, GenesisReceiver};
    use primitives::Address;
    use vrrb_core::keypair::KeyPair;

    #[test]
    fn create_single_genesis_txn() {
        let kp = KeyPair::random();
        let mut genesis_config = GenesisConfig::new(Address::new(kp.miner_public_key_owned()));
        generate_genesis_txns(1, kp, &mut genesis_config);
        let contributors: Vec<&GenesisReceiver> = genesis_config
            .receivers
            .iter()
            .filter(|receiver| receiver.is_contributor())
            .collect();
        let investors: Vec<&GenesisReceiver> = genesis_config
            .receivers
            .iter()
            .filter(|receiver| receiver.is_investor())
            .collect();
        assert_eq!(contributors.len(), 0);
        assert_eq!(investors.len(), 1);
    }

    #[test]
    fn create_odd_number_of_genesis_txns() {
        let kp = KeyPair::random();
        let mut genesis_config = GenesisConfig::new(Address::new(kp.miner_public_key_owned()));
        generate_genesis_txns(3, kp, &mut genesis_config);
        let contributors: Vec<&GenesisReceiver> = genesis_config
            .receivers
            .iter()
            .filter(|receiver| receiver.is_contributor())
            .collect();
        let investors: Vec<&GenesisReceiver> = genesis_config
            .receivers
            .iter()
            .filter(|receiver| receiver.is_investor())
            .collect();
        assert_eq!(contributors.len(), 1);
        assert_eq!(investors.len(), 2);
    }

    #[test]
    fn create_even_number_of_genesis_txns() {
        let kp = KeyPair::random();
        let mut genesis_config = GenesisConfig::new(Address::new(kp.miner_public_key_owned()));
        generate_genesis_txns(4, kp, &mut genesis_config);
        let contributors: Vec<&GenesisReceiver> = genesis_config
            .receivers
            .iter()
            .filter(|receiver| receiver.is_contributor())
            .collect();
        let investors: Vec<&GenesisReceiver> = genesis_config
            .receivers
            .iter()
            .filter(|receiver| receiver.is_investor())
            .collect();
        assert_eq!(contributors.len(), 2);
        assert_eq!(investors.len(), 2);
    }
}
