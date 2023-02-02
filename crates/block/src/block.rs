// This file contains code for creating blocks to be proposed, including the
// genesis block and blocks being mined.

use std::fmt;

use primitives::{types::SecretKey as SecretKeyBytes, Epoch};
use reward::reward::Reward;
#[cfg(mainnet)]
use reward::reward::GENESIS_REWARD;
use ritelinked::LinkedHashMap;
use secp256k1::{
    hashes::{sha256 as s256, Hash},
    Message,
};
use serde::{Deserialize, Serialize};
use sha256::digest;
use utils::{create_payload, hash_data};
use vrrb_core::{
    accountable::Accountable,
    claim::Claim,
    keypair::KeyPair,
    txn::Txn,
    verifiable::Verifiable,
};

#[cfg(mainnet)]
use crate::genesis;
use crate::{
    genesis,
    header::BlockHeader,
    invalid::{BlockError, InvalidBlockErrorReason},
    ConvergenceBlock,
    GenesisBlock,
    ProposalBlock,
};

pub trait InnerBlock {
    type Header;
    type RewardType;

    fn get_header(&self) -> Self::Header;
    fn get_next_block_seed(&self) -> u64;
    fn get_next_block_reward(&self) -> Self::RewardType;
    fn is_genesis(&self) -> bool;
    fn get_hash(&self) -> String;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(C)]
pub enum Block {
    Convergence { block: ConvergenceBlock },
    Proposal { block: ProposalBlock },
    Genesis { block: GenesisBlock },
}

impl Block {
    pub fn is_convergence(&self) -> bool {
        matches!(self, Block::Convergence { .. })
    }

    pub fn is_proposal(&self) -> bool {
        matches!(self, Block::Proposal { .. })
    }

    pub fn is_genesis(&self) -> bool {
        matches!(self, Block::Genesis { .. })
    }
}

impl fmt::Display for ConvergenceBlock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "ConvergenceBlock(\n \
            header: {:?},\n",
            self.header
        )
    }
}

//TODO: impl fmt::Display for ProposalBlock & GenesisBlock
impl From<ConvergenceBlock> for Block {
    fn from(block: ConvergenceBlock) -> Block {
        Block::Convergence { block }
    }
}

impl From<ProposalBlock> for Block {
    fn from(block: ProposalBlock) -> Block {
        Block::Proposal { block }
    }
}

impl From<GenesisBlock> for Block {
    fn from(block: GenesisBlock) -> Block {
        Block::Genesis { block }
    }
}

impl InnerBlock for ConvergenceBlock {
    type Header = BlockHeader;
    type RewardType = Reward;

    fn get_header(&self) -> Self::Header {
        self.header.clone()
    }

    fn get_next_block_seed(&self) -> u64 {
        self.get_header().next_block_seed
    }

    fn get_next_block_reward(&self) -> Self::RewardType {
        self.get_header().next_block_reward
    }

    fn is_genesis(&self) -> bool {
        false
    }

    fn get_hash(&self) -> String {
        self.hash.clone()
    }
}

impl InnerBlock for GenesisBlock {
    type Header = BlockHeader;
    type RewardType = Reward;

    fn get_header(&self) -> Self::Header {
        self.header.clone()
    }

    fn get_next_block_seed(&self) -> u64 {
        self.get_header().next_block_seed
    }

    fn get_next_block_reward(&self) -> Self::RewardType {
        self.get_header().next_block_reward
    }

    fn is_genesis(&self) -> bool {
        true
    }

    fn get_hash(&self) -> String {
        self.hash.clone()
    }
}
