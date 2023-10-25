use primitives::Address;
use ritelinked::LinkedHashMap;
use secp256k1::Message;
use vrrb_core::{
    account::Account,
    keypair::Keypair,
    transactions::{NewTransferArgs, Transaction, TransactionDigest, TransactionKind, Transfer},
};

#[deprecated = "outdated vesting"]
// 50% after one year, then monthly for 12 months
const CONTRIBUTOR_VESTING: VestingConfig = VestingConfig {
    cliff_fraction: 0.5f64,
    cliff_years: 1f64,
    unlocks: 12,
    unlock_years: 1f64,
};

#[deprecated = "outdated vesting"]
// 25% after half year, then monthly for 18  months
const INVESTOR_VESTING: VestingConfig = VestingConfig {
    cliff_fraction: 0.25f64,
    cliff_years: 0.75f64,
    unlocks: 18,
    unlock_years: 1.5f64,
};

#[derive(Debug, Clone)]
#[deprecated = "outdated vesting model"]
pub struct VestingConfig {
    pub cliff_fraction: f64,
    pub cliff_years: f64,
    pub unlocks: usize,
    pub unlock_years: f64,
}

#[derive(Debug, Clone, PartialEq)]
#[deprecated = "replaced by block::GenesisReceiver, no longer necessary since we are not using this model"]
pub enum GenesisReceiverKind {
    Investor,
    Contributor,
}

#[derive(Debug, Clone)]
#[deprecated = "replaced by block::GenesisReceiver"]
pub struct GenesisReceiver {
    pub address: Address,
    pub genesis_receiver_kind: GenesisReceiverKind,
    pub vesting_config: Option<VestingConfig>,
}

impl GenesisReceiver {
    pub fn new(
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
    pub fn create_investor(address: &Address) -> Self {
        Self::new(
            address.to_owned(),
            GenesisReceiverKind::Investor,
            INVESTOR_VESTING,
        )
    }
    pub fn create_contributor(address: &Address) -> Self {
        Self::new(
            address.to_owned(),
            GenesisReceiverKind::Contributor,
            CONTRIBUTOR_VESTING,
        )
    }
}

#[derive(Debug, Clone)]
#[deprecated = "replaced by block::GenesisConfig"]
pub struct GenesisConfig {
    sender: Address,
    receivers: Vec<GenesisReceiver>,
}
impl GenesisConfig {
    pub fn new(sender: Address, receivers: Vec<GenesisReceiver>) -> Self {
        Self { sender, receivers }
    }
    pub fn receivers(&self) -> &[GenesisReceiver] {
        self.receivers.as_ref()
    }
    pub fn sender(&self) -> &Address {
        &self.sender
    }
}
#[deprecated = "outdated vesting model will be replaced"]
pub fn create_vesting(
    sender_keypair: &Keypair,
    sender_address: &Address,
    genesis_receiver: &GenesisReceiver,
) -> (TransactionDigest, TransactionKind) {
    let tx_kind = create_txn_from_addresses(
        sender_keypair,
        sender_address,
        &genesis_receiver.address,
        vec![],
    );
    (tx_kind.id(), tx_kind)
}
#[deprecated = "replaced by genesis rewards"]
pub fn generate_genesis_txns(
    sender_keypair: Keypair,
    genesis_config: &GenesisConfig,
) -> LinkedHashMap<TransactionDigest, TransactionKind> {
    genesis_config
        .receivers()
        .iter()
        .map(|genesis_receiver| {
            create_vesting(&sender_keypair, genesis_config.sender(), genesis_receiver)
        })
        .collect()
}

fn create_txn_from_addresses(
    sender_keypair: &Keypair,
    sender: &Address,
    receiver: &Address,
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
        sender_address: sender.clone(),
        sender_public_key: *pk,
        receiver_address: receiver.clone(),
        token: None,
        amount,
        signature: sk
            .sign_ecdsa(Message::from_hashed_data::<secp256k1::hashes::sha256::Hash>(b"vrrb")),
        validators: Some(validators),
        nonce: sender_acct.nonce() + 1,
    };

    let mut txn = TransactionKind::Transfer(Transfer::new(txn_args));

    txn.sign(sk);

    txn
}
