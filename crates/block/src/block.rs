// This file contains code for creating blocks to be proposed, including the
// genesis block and blocks being mined.

use std::fmt::{self, Debug};

use bulldag::vertex::Vertex;
use reward::reward::Reward;
#[cfg(mainnet)]
use reward::reward::GENESIS_REWARD;
use serde::{Deserialize, Serialize};

#[cfg(mainnet)]
use crate::genesis;
use crate::{header::BlockHeader, ConvergenceBlock, GenesisBlock, ProposalBlock};

pub trait InnerBlock: std::fmt::Debug + Send {
    type Header;
    type RewardType;

    fn get_header(&self) -> Self::Header;
    fn get_next_block_seed(&self) -> u64;
    fn get_next_block_reward(&self) -> Self::RewardType;
    fn is_genesis(&self) -> bool;
    fn get_hash(&self) -> String;
    fn get_ref_hashes(&self) -> Vec<String>;
    fn as_static_convergence(&self) -> Option<ConvergenceBlock>;
    fn as_static_genesis(&self) -> Option<GenesisBlock>;
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
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

    pub fn size(&self) -> usize {
        match self {
            Block::Convergence { block } => block
                .txns
                .iter()
                .map(|(_, set)| set)
                .map(std::mem::size_of_val)
                .sum(),

            Block::Proposal { block } => block
                .txns
                .iter()
                .map(|(_, set)| set)
                .map(std::mem::size_of_val)
                .sum(),

            Block::Genesis { block } => block
                .txns
                .iter()
                .map(|(_, set)| set)
                .map(std::mem::size_of_val)
                .sum(),
        }
    }

    pub fn hash(&self) -> String {
        match self {
            Block::Convergence { block } => block.hash.clone(),
            Block::Proposal { block } => block.hash.clone(),
            Block::Genesis { block } => block.hash.clone(),
        }
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

    fn get_ref_hashes(&self) -> Vec<String> {
        self.header.ref_hashes.clone()
    }

    fn as_static_convergence(&self) -> Option<ConvergenceBlock> {
        Some(self.clone())
    }

    fn as_static_genesis(&self) -> Option<GenesisBlock> {
        None
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

    fn get_ref_hashes(&self) -> Vec<String> {
        self.header.ref_hashes.clone()
    }

    fn as_static_convergence(&self) -> Option<ConvergenceBlock> {
        None
    }

    fn as_static_genesis(&self) -> Option<GenesisBlock> {
        Some(self.clone())
    }
}

impl From<Block> for Vertex<Block, String> {
    fn from(item: Block) -> Vertex<Block, String> {
        match item {
            Block::Convergence { ref block } => Vertex::new(item.clone(), block.hash.clone()),
            Block::Proposal { ref block } => Vertex::new(item.clone(), block.hash.clone()),
            Block::Genesis { ref block } => Vertex::new(item.clone(), block.hash.clone()),
        }
    }
}
