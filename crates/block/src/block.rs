// This file contains code for creating blocks to be proposed, including the
// genesis block and blocks being mined.

use std::fmt::{self, Debug, Formatter};

use bulldag::vertex::Vertex;
use reward::reward::Reward;
#[cfg(mainnet)]
use reward::reward::GENESIS_REWARD;
use serde::{Deserialize, Serialize};
use vrrb_core::transactions::Transaction;

#[cfg(mainnet)]
use crate::genesis;
use crate::{header::BlockHeader, ConvergenceBlock, GenesisBlock, ProposalBlock};

pub trait InnerBlock<T>: std::fmt::Debug + Send {
    type Header;
    type RewardType;

    fn get_header(&self) -> Self::Header;
    fn get_next_block_seed(&self) -> u64;
    fn get_next_block_reward(&self) -> Self::RewardType;
    fn is_genesis(&self) -> bool;
    fn get_hash(&self) -> String;
    fn get_ref_hashes(&self) -> Vec<String>;
    fn as_static_convergence(&self) -> Option<ConvergenceBlock>;
    fn as_static_genesis(&self) -> Option<GenesisBlock<T>>;
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
#[repr(C)]
pub enum Block<T> {
    Convergence { block: ConvergenceBlock },
    Proposal { block: ProposalBlock<T> },
    Genesis { block: GenesisBlock<T> },
}

impl<'a, T: Transaction<'a>> Block<T> {
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
impl<'a, T: Transaction<'a>> From<ConvergenceBlock> for Block<T> {
    fn from(block: ConvergenceBlock) -> Block<T> {
        Block::Convergence { block }
    }
}

impl<'a, T: Transaction<'a>> From<ProposalBlock<T>> for Block<T> {
    fn from(block: ProposalBlock<T>) -> Block<T> {
        Block::Proposal { block }
    }
}

impl<'a, T: Transaction<'a>> From<GenesisBlock<T>> for Block<T> {
    fn from(block: GenesisBlock<T>) -> Block<T> {
        Block::Genesis { block }
    }
}

impl<'a, T: Transaction<'a>> InnerBlock<T> for ConvergenceBlock {
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

    fn as_static_convergence(&self) -> Option<ConvergenceBlock> {
        Some(self.clone())
    }

    fn as_static_genesis(&self) -> Option<GenesisBlock<T>> {
        None
    }

    fn get_ref_hashes(&self) -> Vec<String> {
        self.header.ref_hashes.clone()
    }
}

impl<'a, T: Transaction<'a>> Debug for GenesisBlock<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl<'a, T: Transaction<'a>> InnerBlock<T> for GenesisBlock<T> {
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

    fn as_static_genesis(&self) -> Option<GenesisBlock<T>> {
        Some(self.clone())
    }
}

impl<'a, T: Transaction<'a> + Clone + Debug> From<Block<T>> for Vertex<Block<T>, String> {
    fn from(item: Block<T>) -> Vertex<Block<T>, String> {
        match item {
            Block::Convergence { ref block } => Vertex::new(item.clone(), block.hash.clone()),
            Block::Proposal { ref block } => Vertex::new(item.clone(), block.hash.clone()),
            Block::Genesis { ref block } => Vertex::new(item.clone(), block.hash.clone()),
        }
    }
}
