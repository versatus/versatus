//Generate verifiably random quorum seed using last block hash

use blockchain::blockchain::Blockchain;
use state::state::Components;

pub trait ELECTION{ 
    fn elect_quorum(&mut self, blockchain: &Blockchain) -> Vec<Staker>;
}

 