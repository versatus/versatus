//Generate verifiably random quorum seed using last block hash

use blockchain::blockchain::Blockchain;
use state::state::Components;
use crate::quorum::Quorum;
use miner::miner::Miner;

pub trait Election{ 
    fn elect_quorum(&mut self, blockchain: &Blockchain) -> Quorum;
    fn get_current_quorum(&self) -> Quorum;
    fn re_try_election(&mut self, blockchain: &Blockchain) -> Quorum;
}

 