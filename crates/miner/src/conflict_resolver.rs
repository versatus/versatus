use std::collections::BTreeMap;

use ethereum_types::U256;

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
/// use std::collections::BTreeMap;
///
/// use ethereum_types::U256;
///
///
/// pub trait Resolver {
///     type Proposal;
///     type Identified;
///     type Source;
///     type BallotInfo;
///
///     fn identify(&self, proposals: &Vec<Self::Proposal>) -> Self::Identified;
///     fn resolve(&self, proposals: &Vec<Self::Proposal>, round: u128) -> Vec<Self::Proposal>;
///     fn resolve_earlier(
///         &self,
///         proposals: &Vec<Self::Proposal>,
///         round: u128,
///     ) -> Vec<Self::Proposal>;
///     fn get_sources(&self, proposals: &Vec<Self::Proposal>) -> Vec<Self::Proposal>;
///     fn get_election_results(
///         &self,
///         proposers: &Vec<Self::Proposal>,
///     ) -> BTreeMap<U256, Self::BallotInfo>;
///     fn get_proposers(&self, proposals: &Vec<Self::Proposal>) -> Vec<Self::BallotInfo>;
///     fn append_winner(
///         &self,
///         conflicts: &mut Self::Identified,
///         election_results: &mut BTreeMap<U256, Self::BallotInfo>,
///     );
///     fn resolve_current(&self, current: &mut Vec<Self::Proposal>, conflicts: &Self::Identified);
///     fn split_proposals_by_round(
///         &self,
///         proposals: &Vec<Self::Proposal>,
///     ) -> (Vec<Self::Proposal>, Vec<Self::Proposal>) {
///         (vec![], vec![])
///     }
/// }
/// ```
// TODO: This should be moved to a separate crate
// TODO: We should add a basic doctest example of implementing this
pub trait Resolver {
    type Proposal;
    type Identified;
    type Source;
    type BallotInfo;

    fn identify(&self, proposals: &[Self::Proposal]) -> Self::Identified;
    fn resolve(&self, proposals: &[Self::Proposal], round: u128, seed: u64) -> Vec<Self::Proposal>;
    fn resolve_earlier(&self, proposals: &[Self::Proposal], round: u128) -> Vec<Self::Proposal>;
    fn get_sources(&self, proposals: &Self::Proposal) -> Vec<Self::Source>;
    fn get_election_results(
        &self,
        proposers: &[Self::BallotInfo],
        seed: u64,
    ) -> BTreeMap<U256, Self::BallotInfo>;
    fn get_proposers(&self, proposals: &[Self::Proposal]) -> Vec<Self::BallotInfo>;
    fn append_winner(
        &self,
        conflicts: &mut Self::Identified,
        election_results: &mut BTreeMap<U256, Self::BallotInfo>,
    );
    fn resolve_current(&self, current: &mut Vec<Self::Proposal>, conflicts: &Self::Identified);
    fn split_proposals_by_round(
        &self,
        proposals: &[Self::Proposal],
    ) -> (Vec<Self::Proposal>, Vec<Self::Proposal>) {
        let _ = proposals;
        (vec![], vec![])
    }
}
