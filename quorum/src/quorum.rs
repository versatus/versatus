 use crate::election::ELECTION;
 use blockchain::blockchain::Blockchain;
 use state::state::Components;
 use block::block::Block;
 use std::fmt::{Display};
 use vrrb_vrf::{vvrf::VVRF, vrng::VRNG};
 use rand_chacha::{ChaCha20Rng};
 use std::u32::MAX;
 use std::u64::MAX as u64MAX;
 use crate::lr_trie::{LeftRightTrie};

 #[derive(Debug)]
pub enum InvalidQUORUM{
    InvalidSeedError, 
    InvalidElectionError,
}

impl Display for InvalidQUORUM {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvalidQUORUM::InvalidSeedError => write!(f, "Invalid seed"),
            InvalidQUORUM::InvalidElectionError => write!(f, "Invalid election"),
        }
    
    }
}

impl std::error::Error for InvalidQUORUM {}
 
 pub struct QUORUM{
    pub quorum_seed: String,
    pub pointer_sums: Vec<u64>,
    pub masternodes: Vec<String>,
    pub quorum_pk: String,
    pub election_block_height: u64,
    pub election_timestamp: u64,
 }

 impl ELECTION for QUORUM{

   fn generate_quorum_seed(&mut self, components: &Components) -> u64{
      let mut block_bytes = (components.child).unwrap();
      //need to add if let Some check for child
      let child_block = Block::from_bytes(&mut block_bytes);
      let child_block_hash = child_block.hash;
      let sk = VVRF::generate_secret_key();
      let vvrf = VVRF::new(child_block_hash.as_bytes(), sk);
   
      assert!(VVRF::verify_seed(&mut vvrf).is_ok());
      
      let rng: u64 = vvrf.generate_u64();
      //is u32::MAX inclusive?
      assert!(rng < u64MAX);

      return rng;
   }

   fn calculate_pointer_sums(&mut self, blockchain: &Blockchain) -> Vec<u64> {
      //needs to be replaced by getter/iter in LRTrie
      let mut pointer_sums: Vec<u64> = Vec::new();
      let mut sum: u64 = 0;
      for i in 0..blockchain.len() {
         let block = blockchain.get_block(i);
         sum += block.pointer;
         pointer_sums.push(sum);
      }
      return pointer_sums;   
   }

   fn elect_masternodes(&mut self, blockchain: &Blockchain) -> Vec<Staker> {

   }
 }

 
 impl QUORUM{
   pub fn new() -> QUORUM{
      let seed = QUORUM::generate_quorum_seed();

      

      QUORUM{

         quorum_seed: String::new(),
         pointer_sums: Vec::new(),
         masternodes: Vec::new(),
         quorum_pk: String::new(),
         election_block_height: 0,
         election_timestamp: 0,
      }
         /*
         et mut vrf = VVRF::generate_vrf(CipherSuite::SECP256K1_SHA256_TAI);
        let pubkey = VVRF::generate_pubkey(&mut vrf, sk);
        let (proof, hash) = VVRF::generate_seed(&mut vrf, message, sk).unwrap();
        ///rng calculated from hash
        let rng = ChaCha20Rng::from_seed(hash);
        ///populate VVRF fields
        VVRF {
            vrf,
            pubkey,
            message: message.to_vec(),
            proof,
            hash,
            rng: rng,
        }
        */
         
      }
   }
   fn calculate_pointer_sums(&mut self, blockchain: &Blockchain) -> Vec<u64> {
      let mut pointer_sums: Vec<u64> = Vec::new();
      let mut sum: u64 = 0;
      for i in 0..blockchain.len() {
         let block = blockchain.get_block(i);
         sum += block.pointer;
         pointer_sums.push(sum);
      }
      return pointer_sums;   
   }

 }
 