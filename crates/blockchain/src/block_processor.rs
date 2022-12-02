use std::collections::{HashSet, LinkedList};

use crate::result::{ChainError, Result};
use block::{header::BlockHeader, Block};
use ritelinked::LinkedHashMap;
use state::NetworkState;
use telemetry::info;

pub struct BlockProcessor {
    genesis: Option<Block>,
    child: Option<Block>,
    parent: Option<Block>,
    chain: LinkedList<BlockHeader>,
    chain_db: String, // Path to the chain database.
    block_cache: LinkedHashMap<String, Block>,
    future_blocks: LinkedHashMap<String, Block>,
    invalid: LinkedHashMap<String, Block>,
    // components_received: HashSet<ComponentTypes>,
    updating_state: bool,
    processing_backlog: bool,
    started_updating: Option<u128>,
    state_update_cache: LinkedHashMap<u128, LinkedHashMap<u128, Vec<u8>>>,
}

impl BlockProcessor {
    pub fn new() -> Self {
        Self {
            genesis: todo!(),
            child: todo!(),
            parent: todo!(),
            chain: todo!(),
            chain_db: todo!(),
            block_cache: todo!(),
            future_blocks: todo!(),
            invalid: todo!(),
            updating_state: todo!(),
            processing_backlog: todo!(),
            started_updating: todo!(),
            state_update_cache: todo!(),
        }
    }

    /// Processes a block and returns either a result (Ok(()) if the block is
    /// valid, InvalidBlockError if not)
    pub fn process_block(
        &mut self,
        network_state: &NetworkState,
        // _reward: &Reward,
        block: &Block,
    ) -> Result<()> {
        // Check if block is in sequence
        self.validate_block_sequence(block)?;

        // Workflow is as follows:
        // check whether the block is in sequence or not
        // check whether the block is malformed
        // check whether the block is valid with respect to a known genesis block

        if let Some(genesis_block) = &self.genesis {
            if let Some(last_block) = &self.child {
                if let Err(e) = block.valid(last_block, &(network_state.to_owned())) {
                    self.future_blocks
                        .insert(block.clone().header.last_hash, block.clone());
                    Err(e)
                } else {
                    self.parent = self.child.clone();
                    self.child = Some(block.clone());
                    self.chain.push_back(block.header.clone());
                    if self.block_cache.len() == 100 {
                        self.block_cache.pop_back();
                        self.block_cache.insert(block.hash.clone(), block.clone());
                    }

                    if let Err(e) = self.dump(block) {
                        info!("Error dumping block to chain db: {:?}", e);
                    };
                    Ok(())
                }
            } else if let Err(e) = block.valid(genesis_block, &(network_state.to_owned())) {
                Err(e)
            } else {
                self.child = Some(block.clone());
                self.chain.push_back(block.header.clone());
                if let Err(e) = self.dump(block) {
                    info!("Error dumping block to chain db: {:?}", e);
                };
                Ok(())
            }
        } else {
            // check that this is a valid genesis block.
            if block.header.block_height == 0 {
                if let Ok(true) = block.valid_genesis(&(network_state.to_owned())) {
                    self.genesis = Some(block.clone());
                    self.child = Some(block.clone());
                    self.block_cache.insert(block.hash.clone(), block.clone());
                    self.chain.push_back(block.header.clone());
                    if let Err(e) = self.dump(block) {
                        info!("Error dumping block to chain db: {:?}", e);
                    };
                    Ok(())
                } else {
                    self.invalid.insert(block.hash.clone(), block.clone());
                    Err(ChainError::General)
                }
            } else {
                // request genesis block.
                self.future_blocks
                    .insert(block.clone().header.last_hash, block.clone());
                Err(ChainError::BlockOutOfSequence)
            }
        }
    }

    pub fn store_block() {
        //
    }

    // TODO: Discuss whether some of, or everything from here down should be moved
    // to a separate module for: a. readability
    // b. efficiency
    // c. to better organize similar functionality

    /// Checks whether the block is in sequence or not.
    pub fn validate_block_sequence(&self, block: &Block) -> Result<bool> {
        if self.genesis.clone().is_some() {
            if let Some(child) = self.child.clone() {
                let next_height = child.header.block_height + 1;
                if block.header.block_height > next_height {
                    //I'm missing blocks return BlockOutOfSequence error

                    Err(ChainError::BlockOutOfSequence)
                } else if block.header.block_height < next_height {
                    Err(ChainError::NotTallestChain)
                } else {
                    Ok(true)
                }
            } else if block.header.block_height > 1 {
                Err(ChainError::BlockOutOfSequence)
            } else if block.header.block_height < 1 {
                Err(ChainError::NotTallestChain)
            } else {
                Ok(true)
            }
        } else if block.header.block_height != 0 {
            Err(ChainError::BlockOutOfSequence)
        } else {
            Ok(true)
        }
    }

    /// Checks if the next block height is valid, i.e. +1 as compared to
    /// previous block.
    pub fn validate_block_height(&self, block: &Block) -> bool {
        // Check if there is a genesis block
        if self.genesis.as_ref().is_some() {
            // If so, check if there is a child block
            if let Some(child) = self.child.as_ref() {
                // If so check if the block height is equal to last block's height + 1
                if child.header.block_height + 1 != block.header.block_height {
                    // If not, then return false (invalid height)
                    return false;
                }
            } else {
                // otherwise check if the block height is one
                // if not, return false (invalid height)
                if block.header.block_height != 1 {
                    return false;
                }
            }
        } else {
            // If there is no genesis block, then check if the block height is 0
            // if not, return false
            if block.header.block_height != 0 {
                return false;
            }
        }

        true
    }

    /// Puts blocks into an ordered map to process later in the event that the
    /// chain is updating the state.
    pub fn stash_future_blocks(&mut self, block: &Block) {
        self.future_blocks
            .insert(block.clone().header.last_hash, block.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn should_validate_a_block() {}

    fn should_return_false_if_block_height_is_invalid() {
        let block_processor = BlockProcessor {};

        block_processor.validate_block_height();
    }
}
