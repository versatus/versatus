use std::u32::MAX as u32MAX;

use claim::claim::Claim;
use thiserror::Error;
use vrrb_vrf::{vrng::VRNG, vvrf::VVRF};

use crate::election::Election;

#[derive(Error, Debug)]
pub enum InvalidQuorum {
    #[error("inavlid seed generated")]
    InvalidSeedError(),

    #[error("invalid pointer sum")]
    InvalidPointerSumError(Vec<Claim>),

    #[error("invalid child block")]
    InvalidChildBlockError(),

    #[error("not enough eligible nodes")]
    InsufficientNodesError(),
}

pub struct Quorum {
    pub quorum_seed: u128,
    pub master_pubkeys: Vec<String>,
    pub quorum_pk: String,
    pub election_block_height: u128,
    pub election_timestamp: u128,
}

type Timestamp = u128;
type Height = u128;
type BlockHash = String;

impl Election for Quorum {
    type Ballot = Vec<Claim>;
    type Error = InvalidQuorum;
    type Payload = (Timestamp, Height, BlockHash);
    type Return = Self;

    fn elect_quorum(
        &mut self,
        payload: Self::Payload,
        ballot: Self::Ballot,
    ) -> Result<&Self::Return, Self::Error> {
        let quorum_seed = match self.generate_quorum_seed(payload.0, payload.1, payload.2) {
            Ok(quorum_seed) => quorum_seed,
            Err(e) => return Err(e),
        };
        self.quorum_seed = quorum_seed;

        let eligible_claims = match Quorum::get_eligible_claims(ballot) {
            Ok(eligible_claims) => eligible_claims,
            Err(e) => return Err(e),
        };

        let elected_quorum = match self.get_final_quorum(quorum_seed, eligible_claims) {
            Ok(elected_quorum) => elected_quorum,
            Err(e) => return Err(e),
        };
        return Ok(elected_quorum);
    }

    fn run_election(
        &mut self,
        payload: Self::Payload,
        ballot: Self::Ballot,
    ) -> Result<&Self::Return, Self::Error> {
        match self.elect_quorum(payload, ballot) {
            Ok(quorum) => return Ok(quorum),
            Err(e) => return Err(e),
        };
    }
}

//result enum for errors
impl Quorum {
    //make new generate a blank/default quorum like a constructor
    pub fn new() -> Quorum {
        return Quorum {
            quorum_seed: 0,
            master_pubkeys: Vec::new(),
            quorum_pk: String::new(),
            election_block_height: 0,
            election_timestamp: 0,
        };
    }

    pub fn generate_quorum_seed(
        &mut self,
        timestamp: Timestamp,
        height: Height,
        block_hash: BlockHash,
    ) -> Result<u128, InvalidQuorum> {
        if height == 0 {
            return Err(InvalidQuorum::InvalidChildBlockError());
        } else if timestamp == 0 {
            return Err(InvalidQuorum::InvalidChildBlockError());
        } else {
            let sk = VVRF::generate_secret_key();
            let mut vvrf = VVRF::new(block_hash.as_bytes(), sk);

            if VVRF::verify_seed(&mut vvrf).is_err() {
                return Err(InvalidQuorum::InvalidSeedError());
            }

            let mut random_number = vvrf.generate_u64();
            while random_number < u32MAX as u64 {
                random_number = vvrf.generate_u64();
            }

            self.quorum_seed = random_number as u128;
            self.election_timestamp = timestamp;
            self.election_block_height = height;

            return Ok(random_number as u128);
        }
    }

    pub fn get_eligible_claims(claims: Vec<Claim>) -> Result<Vec<Claim>, InvalidQuorum> {
        let mut eligible_claims = Vec::<Claim>::new();
        claims
            .into_iter()
            .filter(|claim| claim.eligible == true)
            .for_each(|claim| {
                eligible_claims.push(claim.clone());
            });

        if eligible_claims.len() < 20 {
            return Err(InvalidQuorum::InsufficientNodesError());
        }
        let eligible_claims = eligible_claims;
        return Ok(eligible_claims);
    }

    pub fn get_final_quorum(
        &mut self,
        quorum_seed: u128,
        claims: Vec<Claim>,
    ) -> Result<&Quorum, InvalidQuorum> {
        let num_claims = ((claims.len() as f32) * 0.51).ceil() as usize;

        let mut claim_tuples: Vec<(Option<u128>, &String)> = claims
            .iter()
            .filter(|claim| claim.get_pointer(quorum_seed) != None)
            .map(|claim| (claim.get_pointer(quorum_seed), &claim.pubkey))
            .collect();

        if claim_tuples.len() < 20 {
            return Err(InvalidQuorum::InvalidPointerSumError(claims));
        }

        claim_tuples.sort_by_key(|claim_tuple| claim_tuple.0.unwrap());

        let pubkeys: Vec<String> = claim_tuples
            .into_iter()
            .map(|claim_tuple| claim_tuple.1.clone())
            .take(num_claims)
            .collect();

        let final_pubkeys = Vec::from_iter(pubkeys[0..num_claims].iter().cloned());
        self.master_pubkeys = final_pubkeys;

        return Ok(self);
    }
}
