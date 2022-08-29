 use crate::election:: Election;
 use blockchain::blockchain::Blockchain;
 use state::state::Components;
 use block::block::Block;
 use std::fmt::{Display};
 use vrrb_vrf::{vvrf::VVRF, vrng::VRNG};
 use rand_chacha::{ChaCha20Rng};
 use std::u32::MAX;
 use std::u64::MAX as u64MAX;
 use lr_trie::{LeftRightTrie};
 use hex::{encode};
 use claim::claim::Claim;
 use indexmap::IndexMap;
 use node::node::Node;
 use crate::dummyNode::DummyNode;
 use rayon::prelude::*;

 #[derive(Debug)]
pub enum InvalidQuorum{
    InvalidSeedError, 
    InvalidElectionError,
    InvalidChildBlockError,
}

impl Display for InvalidQuorum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvalidQuorum::InvalidSeedError => write!(f, "Invalid seed"),
            InvalidQuorum::InvalidElectionError => write!(f, "Invalid election"),
            InvalidQuorum::InvalidChildBlockError => write!(f, "Invalid child block"),
         }
    }
}

impl std::error::Error for InvalidQuorum {}
 
pub struct Quorum{
   pub quorum_seed: u64,
   pub masternodes: Vec<DummyNode>,
   pub quorum_pk: String,
   pub election_block_height: u128,
   pub election_timestamp: u128,
}

 impl Election for Quorum{
   fn elect_quorum(&mut self, blockchain: &Blockchain) -> Quorum{
         return Quorum::new(blockchain);
   }
 }
 
 impl Quorum{
   pub fn new(blockchain: &Blockchain) -> Quorum{
      let quorum_seed = Quorum::generate_quorum_seed(blockchain);

      let child_block = Blockchain::get_child_ref(blockchain);
      let mut child_block_timestamp: u128;
      let mut child_block_height: u128;

      if child_block.is_some() {
         let child_block_hash = child_block.unwrap().hash;
         let child_block_timestamp = child_block.unwrap().header.timestamp;
         let child_block_height = child_block.unwrap().header.block_height;
      } else {
         return Err(InvalidQuorum::InvalidChildBlockError).unwrap();
      }

      let mut dummyNodes: Vec<DummyNode> = Vec::new();
      let node1: DummyNode = DummyNode::new(b"node1");
      let node2: DummyNode = DummyNode::new(b"node2");
      let node3: DummyNode = DummyNode::new(b"node3");

      let mut dummyClaims: Vec<Claim> = Vec::new();
      let addr: String = "0x0000000000000000000000000000000000000000".to_string();
      let claim1: Claim = Claim::new(node1.pubkey, addr, 1);
      let claim2: Claim = Claim::new(node2.pubkey, addr, 2);
      let claim3: Claim = Claim::new(node3.pubkey, addr, 3);


      let masterClaims: Vec<Claim> = Quorum::get_master_claims(dummyClaims);
      let masternodes: Vec<DummyNode> = Quorum::get_quorum_nodes(
         quorum_seed, masterClaims, dummyNodes);


      //rerun when fails --> what is threshold of failure?
      //need 60% of quorum to vite, dont wanna wait until only 60% of quorum is live
      //maybe if 20% of quorum becomes faulty, we do election
      //for now, add failed field, set false, if command from external network
      //and counter of ndoe fail, check if counter meets threshold 
      //the failed to true, rerun
      Quorum{
         quorum_seed,
         masternodes,
         quorum_pk: String::new(),
         election_block_height: child_block_height,
         election_timestamp: child_block_timestamp,
      }
         
   }

   fn generate_quorum_seed(blockchain: &Blockchain) -> u64{

      let child_block = blockchain.get_child_ref();
      
      let mut child_block_hash: String;
      if child_block.is_some() {
         let child_block_hash = child_block.unwrap().hash;
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

   fn get_master_claims(mut claims: Vec<Claim>) -> Vec<Claim> {
      let mut eligible_nodes = Vec::<Claim>::new();
      claims.into_iter().filter(|claim| claim.eligible == true).for_each(
         |claim| {
            eligible_nodes.push(claim.clone());
         }
      );
      return eligible_nodes;  
   }

   fn get_quorum_nodes(
      quorum_seed: u64, 
      claims: Vec<Claim>, 
      nodes: Vec<DummyNode>) -> Vec<DummyNode> {

      
      let claim_tuples: Vec<(Option<u128>, String)> = claims.iter().filter(
         |claim| claim.get_pointer(quorum_seed) != None).map(
         |claim| (claim.get_pointer(quorum_seed), claim.pubkey)
      ).collect();
      

      //make sure no claims didnt match all chars
      if claims.len() > claim_tuples.len(){
         return Err(InvalidQuorum::InvalidElectionError).unwrap();
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

 