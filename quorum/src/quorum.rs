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
 use miner::miner::Miner;
 use claim::claim::Claim;
 use indexmap::IndexMap;
 use node::node::Node;

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
   pub quorum_seed: String,
   pub pointer_sums: Vec<u64>,
   pub masternodes: Vec<String>,
   pub quorum_pk: String,
   pub election_block_height: u128,
   pub election_timestamp: u128,
}

 impl Election for Quorum{

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
      } ;
     
      //rerun when fails --> what is threshold of failure?
      //need 60% of quorum to vite, dont wanna wait until only 60% of quorum is live
      //maybe if 20% of quorum becomes faulty, we do election
      //for now, add failed field, set false, if command from external network
      //and counter of ndoe fail, check if counter meets threshold 
      //the failed to true, rerun
      Quorum{
         quorum_seed,
         pointer_sums: Vec::new(),
         masternodes: Vec::new(),
         quorum_pk: String::new(),
         election_block_height: child_block_height,
         election_timestamp: child_block_timestamp,
      }
         
      }

   fn generate_quorum_seed(blockchain: &Blockchain) -> String{

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

      return encode(rng.to_le_bytes());
   }

   //node_trie: &LeftRightTrie<MINER>
   //let quorum_seed = QUORUM::generate_quorum_seed(components);
   //waiting on node trie to be implemented so traversal is possible and
   //Vec of pointer_sums is returned
   fn calculate_pointer_sum(quorum_seed: String, miner: Miner) -> u64{
      
      let claim_hash = miner.claim.hash;
      let mut pointer_sum: u64 = 0;      
      //needs to be replaced by getter/iter in LRTrie
     
      for (i, char) in quorum_seed.chars().enumerate() {
         let mut claim_index = claim_hash.find(char);
         if claim_index.is_some() {
            let claim_index = claim_index.unwrap() as u64; 
            pointer_sum += claim_index.pow(i as u32);
         }         
      }
      return pointer_sum;   
   }

   fn get_lowest_pointer_nodes(quorum_seed: String, node_trie: &LeftRightTrie<Node>) -> Vec<Miner>{

      //calculate each sum and add to vector and index map, pointing to miner
      let mut sum_to_miner: IndexMap<u64, Vec<Miner>>::new;
      
      //noe trie traversal to isolate miners;
      let mut lowest_pointer_sums = Vec::<u64>::new();

      for miner in node_trie.iter() {
         let current_pointer_sum = Quorum::calculate_pointer_sum(quorum_seed, miner);
         lowest_pointer_sums.push(current_pointer_sum);
         if sum_to_miner.contains_key(current_pointer_sum){
            sum_to_miner.get_mut(&current_pointer_sum).unwrap().push(miner);
         } else {
            sum_to_miner.insert(current_pointer_sum, vec![miner]);
         }
      }

      lowest_pointer_sums.sort(); //
      let num_nodes = ((sum_to_miner.len())/ 0.51).ceil() as u64;
      let mut quorum_nodes: Vec<Miner> = Vec::new();

      let mut i = 0;
      while i < num_nodes + 1 {
         let miners = sum_to_miner.get(lowest_pointer_sums[i as usize]).unwwrap();
         let mut n = 0;
         while n < miners.len() {
            quorum_nodes.push(miners[n]);
            n += 1;
         }
         i += 1;
      }
      //make immutable with binding
      let quorum_nodes = quorum_nodes;
      return quorum_nodes;
   }
 }
 