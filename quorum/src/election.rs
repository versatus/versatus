use claim::claim::Claim;
use crate::quorum::{Quorum, InvalidQuorum};
use crate::dummyNode::DummyNode;
use crate::dummyChildBlock::DummyChildBlock;

pub trait Election{ 
    fn elect_quorum(&mut self, child_block: &DummyChildBlock, claims: Vec<Claim>, nodes: Vec<DummyNode>) -> Result<&Quorum, InvalidQuorum>;
    fn run_election(&mut self, child_block: &DummyChildBlock, claims: Vec<Claim>, nodes: Vec<DummyNode>) -> Result<&Quorum, InvalidQuorum>;
    fn nonce_up_claims(claims: Vec<Claim>) -> Vec<Claim>;
}
