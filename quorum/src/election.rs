//Generate verifiably random quorum seed using last block hash

use blockchain::blockchain::Blockchain;
use crate::quorum::Quorum;

pub trait Election{ 
    fn run_election(){
        let mut quorum = Quorum::new();

        let quorum
        elect_quorum 
        else{
            //defer re-election to consumer 
        }


    }
    //receive claim vec returned by error and return
    fn nonce_up_claims() -> Vec<Claim>;

    fn elect_quorum(&self, claims: Vec<Claim>, nodes: Vec<DummyNode>) -> Result<Quorum, InvalidQuorum>;
}

 