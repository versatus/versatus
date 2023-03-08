use std::collections::HashMap;

use primitives::NodeId;
//use credit_score::CreditScore;
use vrrb_core::claim::CreditScores;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreditModel {
    // one CreditModel per quorum
    pub trusted_peers: HashMap<NodeId, u128>, //pubkeys --> Uuid
    pub final_peer_scores: HashMap<NodeId, u128>,
}

impl CreditModel {
    pub fn new() -> Self {
        return CreditModel {
            trusted_peers: HashMap::new(),
            final_peer_scores: HashMap::new(),
        };
    }

    fn calculate_weights_one_peer(trusted_peers: HashMap<NodeId, u128>, credit_score: CreditScores) -> Vec<u128> {
        //for one peer, turn sij into cij for all its peers
        fn calculate_score(
            current_peer_score: u128,
            peer_scores: CreditScores,
            trusted_peers: HashMap<NodeId, u128>,
        ) -> u128 {
            let sum: u128 = peer_scores.scores_from_peers.clone().values().collect().iter().sum();
            if sum == 0 {
                if trusted_peers[&peer_scores.id] != 0 {
                    return (1 / (trusted_peers.keys().len())) as u128;
                } else {
                    return 0;
                }
            }
            return current_peer_score / (sum - current_peer_score);
        }
        return credit_score
            .scores_from_peers
            .clone()
            .values()
            .collect()
            .iter()
            .map(|x| calculate_score(*x, peer_scores.clone(), trusted_peers.clone()))
            .collect(); //need to collect ids as well
    }

    fn calculate_all_weighted_scores(
        all_peer_scores: Vec<CreditScores>,
    ) -> HashMap<NodeId, Vec<u128>> {
        let mut weighted_scores = HashMap::new();
        for i in 0..(all_peer_scores.len() + 1) {
            weighted_scores.insert(
                all_peer_scores[i].clone().id,
                calculate_weights_one_peer(all_peer_scores[i].clone()),
            );
        }
        return weighted_scores;
    }

    //now, get the final scores (t value)

    fn calculate_final_score_single_peer(trusted_peers: HashMap<NodeId, u128>, final_peer_scores: HashMap<NodeId, u128>, single_peer_weighted_scores: HashMap<NodeId, Vec<u128>>) -> u128 {
        //get h value
        let mut honest_peer_score: &u128;
        let mut peers: Vec<u128> = single_peer_weighted_scores.into_values().collect();
        peers.sort();

        if final_peer_scores.len() == 0 {
            honest_peer_score = peers[0];
        } else {
            let scores: Vec<&u128> = final_peer_scores.clone().values().collect();
            let mut sorted_scores: Vec<&u128> = final_peer_scores.values().clone().collect::<Vec<&u128>>();
            sorted_scores.sort();
            honest_peer_score = sorted_scores[0];
        }

        let mut honest_peer: String = "".to_string();

        //get honest peer id
        let mut peer_ids = single_peer_weighted_scores.clone().keys().collect();

        for i in 0..(peer_ids.len() + 1){
            for j in 0..(single_peer_weighted_scores.keys().len()){
                if &(single_peer_weighted_scores.get(peer_ids[i]).unwrap()[j]) == honest_peer_score {
                    honest_peer = peer_ids[i].to_string(); //is there a better way to check
                }
            }
        }

        let mut proliferation_param: u128;

        if honest_peer_score > &(0.5 as u128) {
            proliferation_param = honest_peer_score.clone();
        } else {
            proliferation_param = (1 - honest_peer_score.clone());
        }

        //get t value
        let mut t_value: u128;
        let trusted_peer_ids: Vec<&String> = trusted_peers.keys().collect();
        let mut total_scores: u128 = 0;
            for i in 0..(peers.len()+1){
                total_scores += peers[i].clone();
            }
        
        if trusted_peer_ids.contains(&&honest_peer) {
            return (1 - proliferation_param) * (total_scores) + (proliferation_param*honest_peer_score);
        } else {
            return proliferation_param * (total_scores) + (1 -proliferation_param) * honest_peer_score;
        }
    }

    pub fn calculate_final_scores(
        &mut self,
        all_credit_scores: Vec<CreditScores> 
    ) -> () {

        //run calculate_final_score_single_peer for each peer in the credit model. collect the t values and update.
        let weighted_scores = calculate_all_weighted_scores(all_credit_scores); //should return a vector of HM <String, u128>


        
        //Self::calculate_final_score_single_peer(self.trusted_peers, self.final_peer_scores);
        //update credit model w new trusted peers and new final peer scores



    }
}
