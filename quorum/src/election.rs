//Generate verifiably random quorum seed using last block hash

use blockchain::blockchain::Blockchain;
use crate::quorum::Quorum;

pub trait Election{ 
    fn elect_quorum(&self, claims: Vec<Claim>, nodes: Vec<DummyNode>) -> Result<Quorum, InvalidQuorum>;
    fn run_election(blockchain: &Blockchain, claims: Vec<Claim>, nodes: Vec<Quorum>) -> Result<Quorum, InvalidQuorum>;
    fn nonce_up_claims(claims: Vec<Claim>) -> Vec<Claim>;
}

 