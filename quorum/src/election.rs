//Generate verifiably random quorum seed using last block hash

use blockchain::blockchain::Blockchain;
use crate::quorum::Quorum;

pub trait Election{ 
    fn run_election(){
        elect_quorum 
        else{
            //defer re-election to consumer 
        }


    }
    //receive claim vec returned by error and return
    fn nonce_up_claims

    fn elect_quorum(&mut self, blockchain: &Blockchain) -> Quorum;
}

 