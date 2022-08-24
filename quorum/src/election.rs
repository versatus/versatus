//Generate verifiably random quorum seed using last block hash

use blockchain::blockchain::Blockchain;
use state::state::Components;
use crate::quorum::QUORUM;
use miner::miner::Miner;

pub trait ELECTION{ 
    fn elect_quorum(&mut self, blockchain: &Blockchain) -> QUORUM;
    fn get_current_quorum(&self) -> QUORUM;
    fn re_try_election(&mut self, blockchain: &Blockchain) -> QUORUM;
}

 