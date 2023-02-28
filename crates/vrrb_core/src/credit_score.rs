//

pub struct Score{
    score: u128, 
    num_txns: u128, 
    pending_scores: Vec<u128>
}

pub struct CreditModel {
    num_peers: u128, //quorum size
    trusted_peers: Vec<String> //pubkeys??
    pub peer_scores: Vec<Vec<Score>> //need number of txns for weighting purposes
}

impl CreditModel{

    pub fn new(num_peers: u128, trusted_peers: Vec<String>) -> Self {
        return CreditModel(num_peers, trusted_peers, Vec::new());

    }

    pub fn calculate_score(individual_score: Score) -> Score {
        let score_sum = individual_score.pending_scores.iter().sum();
        let new_num_txns = individual_score.num_txns + individual_score.pending_scores.len();
        let new_score = (individual_score.score + score_sum) / new_num_txns;
        return Score {new_score, new_num_txns, Vec<u128>::new();}
    }

    calculate_all_scores(&mut self) -> () {
        let new_peer_scores = self.peer_scores.iter().map(|score| calculate_score(score) ).collect();
        self.peer_scores = new_peer_scores;
    } 


    //score_peer

}