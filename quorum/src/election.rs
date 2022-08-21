//Generate verifiably random quorum seed using last block hash

use blockchain::blockchain::Blockchain;
use state::state::Components;

pub trait ELECTION{ 
    fn generate_quorum_seed(&mut self, components: &Components) -> u64;
    fn calculate_pointer_sums(&mut self, blockchain: &Blockchain) -> Vec<u64>;
    fn elect_masternodes(&mut self, blockchain: &Blockchain) -> Vec<String>;
}

 