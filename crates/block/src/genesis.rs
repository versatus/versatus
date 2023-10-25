use primitives::Address;
#[cfg(mainnet)]
use reward::reward::GENESIS_REWARD;
use ritelinked::LinkedHashMap;
use serde::{Deserialize, Serialize};

#[cfg(mainnet)]
use crate::genesis;
use crate::{header::BlockHeader, BlockHash, Certificate, ClaimList};

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct GenesisRewards(pub LinkedHashMap<GenesisReceiver, u128>);

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
#[repr(C)]
pub struct GenesisBlock {
    pub header: BlockHeader,
    pub genesis_rewards: GenesisRewards,
    pub claims: ClaimList,
    pub hash: BlockHash,
    pub certificate: Option<Certificate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct GenesisReceiver(pub Address);
impl GenesisReceiver {
    pub fn new(address: Address) -> Self {
        Self(address)
    }
}

#[derive(Debug, Clone)]
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
