use primitives::{Address, PublicKey, SecretKey};
use ritelinked::LinkedHashMap;
use secp256k1::Secp256k1;
use vrrb_core::{
    keypair::KeyPair,
    transactions::{TransactionDigest, TransactionKind},
};

use crate::genesis;

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

#[derive(Debug, Clone)]
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

pub fn create_vesting(_genesis_receiver: &GenesisReceiver) -> (TransactionDigest, TransactionKind) {
    todo!()
}

pub fn generate_genesis_txns(
    n: usize,
    genesis_config: &mut GenesisConfig,
) -> LinkedHashMap<TransactionDigest, TransactionKind> {
    let mut genesis_txns: LinkedHashMap<TransactionDigest, TransactionKind> =
        LinkedHashMap::with_capacity(n);

    if n != 0 {
        let mut receivers = Vec::with_capacity(n);
        if n == 1 {
            let (_, public_key) = create_genesis_keyset(n);
            let investor = GenesisReceiver::new(
                Address::new(public_key),
                GenesisReceiverKind::Contributor,
                INVESTOR_VESTING,
            );
            let vesting_txn = create_vesting(&investor);
            genesis_txns.insert(vesting_txn.0, vesting_txn.1);
            receivers.push(investor);
        } else {
            let receiver_keysets = create_genesis_keysets(n);
            let lower_half = |n: usize| 0..(n / 2) - 1;
            let upper_half = |n: usize| (n / 2)..;
            let contributor_keysets = &receiver_keysets[lower_half(n)];
            let investor_keysets = &receiver_keysets[upper_half(n)];

            for (_, public_key) in contributor_keysets {
                let contributor = GenesisReceiver::new(
                    Address::new(*public_key),
                    GenesisReceiverKind::Contributor,
                    CONTRIBUTOR_VESTING,
                );
                let vesting_txn = create_vesting(&contributor);
                genesis_txns.insert(vesting_txn.0, vesting_txn.1);
                receivers.push(contributor);
            }
            for (_, public_key) in investor_keysets {
                let investor = GenesisReceiver::new(
                    Address::new(*public_key),
                    GenesisReceiverKind::Contributor,
                    INVESTOR_VESTING,
                );
                let vesting_txn = create_vesting(&investor);
                genesis_txns.insert(vesting_txn.0, vesting_txn.1);
                receivers.push(investor);
            }
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
    (0..n).map(|m| create_genesis_keyset(m)).collect()
}

#[test]
fn create_odd_number_of_genesis_txns() {
    let kp = KeyPair::random();
    let mut genesis_config = GenesisConfig::new(Address::new(kp.miner_public_key_owned()));
    generate_genesis_txns(3, &mut genesis_config);
    dbg!(&genesis_config);
}
