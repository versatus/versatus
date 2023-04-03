use std::collections::HashSet;

use block::{RefHash, Block};
use bulldag::vertex::Vertex;

use crate::conflict_resolver::Resolver;

/// A trait that can be implemented on any type that can build blocks.
/// For current purposes, this is to be implemented on both Miner struct 
/// and Harvester. 
///     
/// ```
/// pub trait BlockBuilder: Resolver {
///     type BlockType;
///     type RefType;
///     
///     fn update(&mut self, adjustment: &i128);
///     fn build(&self) -> Option<Self::BlockType>;
///     fn get_references(&self) -> Vec<Self::RefType>; 
/// }
///
// TODO: This should be moved to a separate crate
pub trait BlockBuilder: Resolver {
    type BlockType;
    type RefType;

    fn update(&mut self, adjustment: &i128); 
    fn build(&self) -> Option<Self::BlockType>;
    fn get_references(&self) -> Option<Vec<Self::RefType>>;

    fn get_orphaned_references(
        &self, 
        idx: RefHash, 
        current_round: usize, 
        n_rounds: usize
    ) -> Vec<Self::RefType> {
        let _ = n_rounds;
        let _ = current_round;
        let _ = idx;
        vec![]
    }

    fn get_last_block_vertex(&self, idx: Option<RefHash>) -> Option<Vertex<Block, String>> {
        let _ = idx;
        None
    }

    fn get_n_rounds_convergence(&self, idx: RefHash, current_round: usize, n_rounds: usize) -> HashSet<RefHash> {
        let _ = idx;
        let _ = current_round;
        let _ = n_rounds;
        vec![]
    }
}
