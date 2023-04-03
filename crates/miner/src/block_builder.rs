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
}
