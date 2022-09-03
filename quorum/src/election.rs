pub trait Election {
    type Return;
    type Error;
    type Ballot;
    type Payload;
 
    fn elect_quorum(&mut self, payload: Self::Payload, ballot: Self::Ballot) -> Result<&Self::Return, Self::Error>;
    fn run_election(&mut self, payload: Self::Payload, ballot: Self::Ballot) -> Result<&Self::Return, Self::Error>;
}

/* 
use claim::claim::Claim;
use crate::quorum::{Quorum, InvalidQuorum};
use crate::dummyNode::DummyNode;
//use crate::dummyChildBlock::DummyChildBlock;

pub trait Election{ 
    fn elect_quorum(&mut self, timestamp: u128, height: u128, hash: String, claims: Vec<Claim>, nodes: Vec<DummyNode>) -> Result<&Quorum, InvalidQuorum>;
    fn run_election(&mut self, timestamp: u128, height: u128, hash: String, claims: Vec<Claim>, nodes: Vec<DummyNode>) -> Result<&Quorum, InvalidQuorum>;
    fn nonce_up_claims(claims: Vec<Claim>) -> Vec<Claim>;
}
*/
//generic type ballots which is claim in quorum
//dont need to check nodes
//remove nonce up claims from election trait; dont worry about it
    //claim itself has a nonce up method

    //in quorum make Payload thruple (u128, u128, String) 