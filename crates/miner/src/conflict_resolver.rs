use std::collections::BTreeMap;

use ethereum_types::U256;
use vrrb_core::claim::Claim;

/// A trait that can be implemented on any type that may need to resolve 
/// any kind of conflict in the process of working. In particular, the 
/// Miner and Harvester should implement this trait. 
///
/// Miners will use the methods to resolve conflicts between 
/// proposal blocks 
///
/// Harvesters will use this trait as an MEV engine to reduce the subset 
/// of transactions that it may want to include in a block to only those 
/// that it has a high probability of winning. 
/// ```
/// pub trait Resolver {
///   type ProposalInner;
///   type Identified: IntoIterator;
///   type SourceInner;
///   type BallotInfo; 
///
///    fn identify<I: IntoIterator<Item = Self::ProposalInner>>(&self, proposals: &I) -> &Self::Identified;
///    fn resolve<I: IntoIterator<Item = Self::ProposalInner>>(&self, proposals: &I) -> &I;
///    fn resolve_earlier<I: IntoIterator<Item = Self::ProposalInner>>(&self, proposals: &I) -> &I;
///    fn get_sources<I: IntoIterator<Item = Self::SourceInner>>(&self, proposals: &I) -> &I;
///    fn get_election_results<I: IntoIterator<Item = Self::BallotInfo>>(&self, proposers: &I) -> BTreeMap<U256, Self::BallotInfo>; 
///    fn get_proposers<I: IntoIterator<Item = Self::ProposalInner>, E: IntoIterator<Item = Self::BallotInfo>>(&self, proposals: &I) -> &E; 
///    fn append_winner(&self, conflicts: &mut Self::Identified, election_results: &mut BTreeMap<U256, Self::BallotInfo>); 
///    fn resolve_current<I: IntoIterator<Item = Self::ProposalInner>>(&self, current: &mut I, conflicts: &Self::Identified);
///    fn split_proposals_by_round<I: IntoIterator<Item = Self::ProposalInner>>(
///        &self, proposals: &I
///    ) -> (I, I) {
///        (vec![], vec![])
///    }
/// }
/// ``` 
///
///
// TODO: This should be moved to a separate crate
// TODO: We should add a basic doctest example of implementing this
pub trait Resolver {
    type ProposalInner;
    type Identified: IntoIterator + Default;
    type SourceInner;
    type BallotInfo;
    
    fn identify<I: IntoIterator<Item = Self::ProposalInner>>(&self, proposals: &I) -> &Self::Identified;
    fn resolve<I: IntoIterator<Item = Self::ProposalInner>>(&self, proposals: &I) -> &I;
    fn resolve_earlier<I: IntoIterator<Item = Self::ProposalInner>>(&self, proposals: &I) -> &I;
    fn get_sources<I: IntoIterator<Item = Self::SourceInner>>(&self, proposals: &I) -> &I;
    fn get_election_results<I: IntoIterator<Item = Self::BallotInfo>>(&self, proposers: &I) -> BTreeMap<U256, Self::BallotInfo>; 
    fn get_proposers<I: IntoIterator<Item = Self::ProposalInner>, E: IntoIterator<Item = Self::BallotInfo>>(&self, proposals: &I) -> &E; 
    fn append_winner(&self, conflicts: &mut Self::Identified, election_results: &mut BTreeMap<U256, Self::BallotInfo>); 
    fn resolve_current(&self, current: &mut Vec<Self::Proposal>, conflicts: &Self::Identified);
    fn split_proposals_by_round(
        &self, proposals: &Vec<Self::Proposal>
    ) -> (Vec<Self::Proposal>, Vec<Self::Proposal>) {
        let _ = proposals;
        (vec![], vec![])
    }
}
