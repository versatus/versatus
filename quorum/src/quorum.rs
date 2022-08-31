 use crate::election:: Election;
 use blockchain::blockchain::Blockchain;
 use block::block::Block;
 use std::fmt::{Display};
 use vrrb_vrf::{vvrf::VVRF, vrng::VRNG};
 use rand_chacha::{ChaCha20Rng};
 use std::u32::MAX;
 use std::u64::MAX as u64MAX;
 use claim::claim::Claim;
 use node::node::Node;
 use crate::dummyNode::DummyNode;
 use rayon::prelude::*;
 use thiserror::Error;


 #[derive(Error, Debug)]
 //add value that caused error
pub enum InvalidQuorum{
   #[error("inavlid seed generated: {}", u64)]
    InvalidSeedError(u64), 

   #[error("invalid pointer sum: {}", Vec<Claim>)]
    InvalidPointerSumError(Vec<Claim>),

   #[error("invalid child block")]
    InvalidChildBlockError(),

    #[error("not enough eligible nodes")]
    InsufficientNodesError(),
}

pub struct Quorum{
   pub quorum_seed: u64,
   pub masternodes: Vec<DummyNode>,
   pub quorum_pk: String,
   pub election_block_height: u128,
   pub election_timestamp: u128,
}

 impl Election for Quorum{
   fn elect_quorum(&self, blockchain: &Blockchain, claims: Vec<Claim>, nodes: Vec<DummyNode>) -> Result<Quorum, InvalidQuorum>{
         
         let quorum_seed = match self.generate_quorum_seed(blockchain) {
            Ok(quorum_seed) => quorum_seed,
            Err(e) => return Err(e),
         }

         let eligible_claims = match Quorum::get_eligible_claims(claims){
            Ok(eligible_claims) => eligible_claims,
            Err(e) => return Err(e),
         }

         let eligible_nodes = match self.get_eligible_nodes(nodes){
            Ok(eligible_nodes) => eligible_nodes,
            Err(e) => return Err(e),
         }
         return self;
   }

   fn nonce_up_claims(claims: Vec<Claim>) -> Vec<Claim>{
      let mut nonce_up_claims = claims.iter().for_each(|claim|{
         let mut nonce_up_claim = claim.clone();
         nonce_up_claim.nonce += 1;
         nonce_up_claims.push(nonce_up_claim);
      });
      return nonce_up_claims;
   }

   fn run_election(blockchain: &Blockchain, claims: Vec<Claim>, nodes: Vec<Quorum>) -> Result<Quorum, InvalidQuorum>{
      let mut quorum = Quorum::new();

      let quorum: Quorum = match quorum.elect_quorum(blockchain, claims, nodes){
         Ok(quorum) => return quorum,
         Err(e) => return Err(e),
      }
   }
 }
 
 //result enum for errors
 impl Quorum{
   //make new generate a blank/default quorum like a constructor
   pub fn new(blockchain: &Blockchain) -> Quorum{
      return Quorum{
         quorum_seed: 0,
         masternodes: Vec::new(),
         quorum_pk: String::new(),
         election_block_height: 0,
         election_timestamp: 0,
      } 
   }

   pub fn generate_quorum_seed(&self, blockchain: &Blockchain) -> Result<Quorum, InvalidQuorum>{

      let child_block_opt = blockchain.get_child_ref();

      let child_block: Block;
      let child_block_timestamp: u128;
      let child_block_height: u128;

      if let Some(block) = child_block_opt {
         child_block_timestamp = block.header.timestamp;
         child_block_height = block.height;
         child_block = block;
      } else {
         return Err(InvalidQuorum::InvalidChildBlockError());
      }
      
      let sk = VVRF::generate_secret_key();
      let vvrf = VVRF::new(child_block.hash.as_bytes(), sk);
   
      assert!(VVRF::verify_seed(&mut vvrf).is_ok());
      
      let rng: u64 = vvrf.generate_u128();
      
      if !rng < u64MAX {
         return Err(InvalidQuorum::InvalidSeedError(rng));
      }

      self.quorum_seed = rng;
      self.election_timestamp = child_block_timestamp;
      self.election_block_height = child_block_height;

      return Ok(rng);
   }

   pub fn get_eligible_claims(mut claims: Vec<Claim>) -> Result<Vec<Claim>, InvalidQuorum> {
      let mut eligible_claims = Vec::<Claim>::new();
      claims.into_iter().filter(|claim| claim.eligible == true).for_each(
         |claim| {
            eligible_claims.push(claim.clone());
         }
      );

      //change to 20 in production
      if eligible_claims.len() >= 5 {
         return Err(InvalidQuorum::InsufficientNodesError());
      }

      let eligible_claims = eligible_claims;

      return Ok(eligible_claims);  
   }

   pub fn get_quorum_nodes(
      &self,
      quorum_seed: u64, 
      claims: Vec<Claim>, 
      nodes: Vec<DummyNode>) -> Result<Quorum, InvalidQuorum> {

      let claim_tuples: Vec<(Option<u128>, String)> = claims.iter().filter(
         |claim| claim.get_pointer(quorum_seed) != None).map(
         |claim| (claim.get_pointer(quorum_seed), claim.pubkey)
      ).collect();
      
      //make sure no claims didnt match all chars
      if claims.len() > claim_tuples.len(){
         return Err(InvalidQuorum::InvalidPointerSumError(claims));
      }
      
      claim_tuples.sort_by_key(|claim_tuple| claim_tuple.0.unwrap());

      let num_nodes =((claim_tuples.len() as f32)/ 0.51).ceil() as u64;

      let mut quorum_nodes: Vec<DummyNode> = Vec::new();

      for node in nodes{
         if quorum_nodes.len() == num_nodes as usize {
            break;
         }
         let node_pubkey = node.pubkey;
         let node_pointer = claim_tuples.iter().find(
            |claim_tuple| claim_tuple.1 == node_pubkey
         ).unwrap().0.unwrap();
         quorum_nodes.push(node);
      }
      let quorum_nodes = quorum_nodes;

      self.masternodes = quorum_nodes;

      return self;
   }

 }

//fewer than 51% w valid pointer sums!
   //integer overflow on u128 (pointer sums are over)
   //get pointer method on claim, if claim hash doesnt match every char in seed, returns none
   //nonce all claims up by 1 and re-run

 