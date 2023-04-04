// This file contains code for creating blocks to be proposed, including the
// genesis block and blocks being mined.


use std::fmt;
use reward::reward::Reward;
#[cfg(mainnet)]
use reward::reward::GENESIS_REWARD;
use serde::{Deserialize, Serialize};

#[cfg(mainnet)]
use crate::genesis;
use crate::{
    header::BlockHeader,
    ConvergenceBlock,
    GenesisBlock,
    ProposalBlock,
};

pub trait InnerBlock: std::fmt::Debug {
    type Header;
    type RewardType;

    fn get_header(&self) -> Self::Header;
    fn get_next_block_seed(&self) -> u64;
    fn get_next_block_reward(&self) -> Self::RewardType;
    fn is_genesis(&self) -> bool;
    fn get_hash(&self) -> String;
    fn get_ref_hashes(&self) -> Vec<String>;
    fn into_static_convergence(&self) -> Option<ConvergenceBlock>;
    fn into_static_genesis(&self) -> Option<GenesisBlock>;
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

    pub fn size(&self) -> usize {
        match self {
            Block::Convergence { block } => block
                .txns
                .iter()
                .map(|(_, set)| set)
                .map(|txn| std::mem::size_of_val(&txn))
                .fold(0, |acc, item| acc + item),

            Block::Proposal { block } => block
                .txns
                .iter()
                .map(|(_, set)| set)
                .map(|txn| std::mem::size_of_val(&txn))
                .fold(0, |acc, item| acc + item),

            Block::Genesis { block } => block
                .txns
                .iter()
                .map(|(_, set)| set)
                .map(|txn| std::mem::size_of_val(&txn))
                .fold(0, |acc, item| acc + item),
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

    fn into_static_convergence(&self) -> Option<ConvergenceBlock> {
        Some(self.clone())
    }
    
    fn into_static_genesis(&self) -> Option<GenesisBlock> {
        None
    }

    fn get_ref_hashes(&self) -> Vec<String> {
        self.header.ref_hashes.clone()
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

    fn into_static_convergence(&self) -> Option<ConvergenceBlock> {
        None
    }
    
    fn into_static_genesis(&self) -> Option<GenesisBlock> {
        Some(self.clone())
    }

    fn get_ref_hashes(&self) -> Vec<String> {
        self.header.ref_hashes.clone()
    }
}

impl From<Block> for Vertex<Block, String> {
    fn from(item: Block) -> Vertex<Block, String> {
        match item {
            Block::Convergence { ref block } => {
                return Vertex::new(item.clone(), block.hash.clone());
            },
            Block::Proposal { ref block } => {
                return Vertex::new(item.clone(), block.hash.clone());
            },
            Block::Genesis { ref block } => {
                return Vertex::new(item.clone(), block.hash.clone());
            }
        }
    }
}
