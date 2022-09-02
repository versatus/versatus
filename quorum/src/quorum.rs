use crate::election:: Election;
use vrrb_vrf::{vvrf::VVRF, vrng::VRNG};
use std::u64::MAX as u64MAX;
use claim::claim::Claim;
use crate::dummyNode::DummyNode;
use crate::dummyChildBlock::DummyChildBlock;
use thiserror::Error;


#[derive(Error, Debug)]
pub enum InvalidQuorum{
  #[error("inavlid seed generated: {0}")]
   InvalidSeedError(u64), 

  #[error("invalid pointer sum")]
   InvalidPointerSumError(Vec<Claim>),

  #[error("invalid child block")]
   InvalidChildBlockError(),

   #[error("not enough eligible nodes")]
   InsufficientNodesError(),
}

pub struct Quorum{
  pub quorum_seed: u128,
  pub masternodes: Vec<DummyNode>,
  pub quorum_pk: String,
  pub election_block_height: u128,
  pub election_timestamp: u128,
}

impl Election for Quorum{
  fn elect_quorum(&mut self, child_block: &DummyChildBlock, claims: Vec<Claim>, nodes: Vec<DummyNode>) -> Result<&Quorum, InvalidQuorum>{
      let quorum_seed = match self.generate_quorum_seed(child_block) {
         Ok(quorum_seed) => quorum_seed,
         Err(e) => return Err(e),
      };
      self.quorum_seed = quorum_seed;

      let eligible_claims = match Quorum::get_eligible_claims(claims){
         Ok(eligible_claims) => eligible_claims,
         Err(e) => return Err(e),
      };
      
      let elected_quorum = match self.get_final_quorum(quorum_seed, eligible_claims, nodes){
         Ok(elected_quorum) => elected_quorum,
         Err(e) => return Err(e),
      };
      return Ok(elected_quorum);
  }


  fn nonce_up_claims(claims: Vec<Claim>) -> Vec<Claim>{
      let mut nonce_up_claims = Vec::new();
      claims.iter().for_each(|claim|{
         let mut nonce_up_claim = claim.clone();
         nonce_up_claim.nonce += 1;
         nonce_up_claims.push(nonce_up_claim);
      });
      return nonce_up_claims;
  }

  fn run_election(&mut self, child_block: &DummyChildBlock, claims: Vec<Claim>, nodes: Vec<DummyNode>) -> Result<&Quorum, InvalidQuorum>{
     match self.elect_quorum(child_block, claims, nodes){
        Ok(quorum) => return Ok(quorum),
        Err(e) => return Err(e),
     };
  }
}

//result enum for errors
impl Quorum{
  //make new generate a blank/default quorum like a constructor
  pub fn new() -> Quorum{
     return Quorum{
        quorum_seed: 0,
        masternodes: Vec::new(),
        quorum_pk: String::new(),
        election_block_height: 0,
        election_timestamp: 0,
     } 
  }

  pub fn generate_quorum_seed(&mut self, child_block: &DummyChildBlock) -> Result<u128, InvalidQuorum>{

     let child_block_timestamp: u128 = child_block.timestamp;
     let child_block_height: u128 = child_block.height;

     if child_block_height == 0{
        return Err(InvalidQuorum::InvalidChildBlockError());
     } else if child_block_timestamp == 0 {
        return Err(InvalidQuorum::InvalidChildBlockError());
     } else {
        let sk = VVRF::generate_secret_key();
        let mut vvrf = VVRF::new(child_block.hash.as_bytes(), sk);
     
        assert!(VVRF::verify_seed(&mut vvrf).is_ok());
        
        let rng: u64 = vvrf.generate_u64();

        if rng > u64MAX {
           return Err(InvalidQuorum::InvalidSeedError(rng));
        }

        self.quorum_seed = rng as u128;
        self.election_timestamp = child_block_timestamp;
        self.election_block_height = child_block_height;

        return Ok(rng as u128);
     }
  }

  pub fn get_eligible_claims(mut claims: Vec<Claim>) -> Result<Vec<Claim>, InvalidQuorum> {
     let mut eligible_claims = Vec::<Claim>::new();
     claims.into_iter().filter(|claim| claim.eligible == true).for_each(
        |claim| {
           eligible_claims.push(claim.clone());
        }
     );

     if eligible_claims.len() < 20 {
        return Err(InvalidQuorum::InsufficientNodesError());
     }
     let eligible_claims = eligible_claims;
     return Ok(eligible_claims);  
  }

  pub fn get_final_quorum(
     &mut self,
     quorum_seed: u128, 
     claims: Vec<Claim>, 
     nodes: Vec<DummyNode>) -> Result<&Quorum, InvalidQuorum> {

     let num_nodes =((claims.len() as f32)* 0.51).ceil() as usize;

     let mut claim_tuples: Vec<(Option<u128>, &String)> = claims.iter().filter(
        |claim| claim.get_pointer(quorum_seed) != None).map(
        |claim| (claim.get_pointer(quorum_seed), &claim.pubkey)
     ).collect();
     
     //change to 20 in production
   if claim_tuples.len() < 20 {
      return Err(InvalidQuorum::InvalidPointerSumError(claims));
     }
     
     claim_tuples.sort_by_key(|claim_tuple| claim_tuple.0.unwrap());

     let mut quorum_nodes: Vec<DummyNode> = Vec::new();
     (0..5).for_each(
         |i| {
            nodes.iter().for_each(
               |node| {
                  if &node.pubkey == claim_tuples[i].1 {
                     quorum_nodes.push(node.clone());
                  }
               }
            );
         }
       );
      
     let quorum_nodes = quorum_nodes;
     self.masternodes = quorum_nodes;
     
     return Ok(self);
  }

}