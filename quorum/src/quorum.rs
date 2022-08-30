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

   #[error("invalid child block: {}", Block)]
    InvalidChildBlockError(Block),
}

pub enum Error {
   #[error("invalid rdo_lookahead_frames {0} (expected < {})", i32::MAX)]
   InvalidLookahead(u32),
}

pub struct Quorum{
   pub quorum_seed: u64,
   pub masternodes: Vec<DummyNode>,
   pub quorum_pk: String,
   pub election_block_height: u128,
   pub election_timestamp: u128,
}

 impl Election for Quorum{
   fn elect_quorum(&mut self, blockchain: &Blockchain) -> Quorum{
         Quorum{

         }
         return Quorum::new(blockchain);
         //move heavy lifting from new() here
         
   }
 }
 
 //result enum for errors
 impl Quorum{
   //make new generate a blank/default quorum like a constructor
   pub fn new(blockchain: &Blockchain) -> Result<Quorum, InvalidQuorum>{
      return Quorum{
         quorum_seed: 0,
         masternodes: Vec::new(),
         quorum_pk: String::new(),
         election_block_height: 0,
         election_timestamp: 0,
      } 
   }

   pub fn generate_quorum_seed(blockchain: &Blockchain) -> u64{

      let child_block = blockchain.get_child_ref();
      
      if let Some(child_block) = child_block {
         child_block_timestamp = child_block.timestamp;
         child_block_height = child_block.height;
      } else {
         return Err(InvalidQuorum::InvalidChildBlockError(child_block));
      }
     
      let sk = VVRF::generate_secret_key();
      let vvrf = VVRF::new(child_block_hash.as_bytes(), sk);
   
      assert!(VVRF::verify_seed(&mut vvrf).is_ok());
      
      let rng: u64 = vvrf.generate_u64();
      //is u32::MAX inclusive?
      assert!(rng < u64MAX);
      assert!(VVRF::verify_seed(&mut vvrf).is_ok());

      return rng;
   }

   pub fn get_eligible_claims(mut claims: Vec<Claim>) -> Vec<Claim> {
      let mut eligible_claims = Vec::<Claim>::new();
      claims.into_iter().filter(|claim| claim.eligible == true).for_each(
         |claim| {
            eligible_claims.push(claim.clone());
         }
      );
      return eligible_claims;  
   }

   pub fn get_quorum_nodes(
      quorum_seed: u64, 
      claims: Vec<Claim>, 
      nodes: Vec<DummyNode>) -> Vec<DummyNode> {

      let claim_tuples: Vec<(Option<u128>, String)> = claims.iter().filter(
         |claim| claim.get_pointer(quorum_seed) != None).map(
         |claim| (claim.get_pointer(quorum_seed), claim.pubkey)
      ).collect();
      
      //make sure no claims didnt match all chars
      if claims.len() > claim_tuples.len(){
         return Err(InvalidQuorum::InvalidElectionError(claims));
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
      return quorum_nodes;
   }
 }

//fewer than 51% w valid pointer sums!
   //integer overflow on u128 (pointer sums are over)
   //get pointer method on claim, if claim hash doesnt match every char in seed, returns none
   //nonce all claims up by 1 and re-run

 