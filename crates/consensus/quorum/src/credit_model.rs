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

    fn calculate_weights_one_peer(&mut self, peer_scores: CreditScores) -> Vec<u128> {
        //for one peer, turn sij into cij for all its peers

        fn calculate_score(
            current_peer_score: u128,
            peer_scores: CreditScores,
            trusted_peers: HashMap<NodeId, u128>,
        ) -> u128 {
            let sum: u128 = peer_scores.peer_score.iter().sum();
            if sum == 0 {
                if trusted_peers[&peer_scores.id] != 0 {
                    return (1 / (trusted_peers.keys().len())) as u128;
                } else {
                    return 0;
                }
            }
            return current_peer_score / (sum - current_peer_score);
        }
        return peer_scores
            .peer_score
            .clone()
            .iter()
            .map(|x| calculate_score(*x, peer_scores.clone(), self.trusted_peers.clone()))
            .collect();
    }

    fn calculate_all_weights(
        &mut self,
        all_peer_scores: Vec<CreditScores>,
    ) -> HashMap<NodeId, Vec<u128>> {
        let mut weighted_scores = HashMap::new();
        for i in 0..(all_peer_scores.len() + 1) {
            weighted_scores.insert(
                all_peer_scores[i].clone().id,
                self.calculate_weights_one_peer(all_peer_scores[i].clone()),
            );
        }
        return weighted_scores;
    }

    //now, get the final scores (t values)

    pub fn calculate_final_scores(weighted_scores: HashMap<NodeId, Vec<u128>>) {}
}
