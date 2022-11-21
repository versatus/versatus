use std::u32::MAX as u32MAX;

use claim::claim::Claim;
use thiserror::Error;
use vrrb_vrf::{vrng::VRNG, vvrf::VVRF};

use crate::election::Election;

///Error type for Quorum
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

    #[error("quorum does not contain a seed")]
    NoSeedError(),
}

///Quorum struct which is created and modified when an election is run
pub struct Quorum {
    pub quorum_seed: u64,
    pub master_pubkeys: Vec<String>,
    pub quorum_pk: String,
    pub election_block_height: u128,
    pub election_timestamp: u128,
}

///generic types from Election trait defined here for Quorums
type Timestamp = u128;
type Height = u128;
type BlockHash = String;
type Seed = u64;

///Payload data comes from current child block
impl Election for Quorum {
    type Ballot = Vec<Claim>;
    type Error = InvalidQuorum;
    type Payload = (Timestamp, Height, BlockHash);
    type Return = Self;
    type Seed = Seed;

    ///a miner calls this fxn to generate a u64 seed for the election using the
    /// vrrb_vrf crate
    fn generate_seed(payload: Self::Payload) -> Result<Seed, InvalidQuorum> {
        if !Quorum::check_payload_validity(payload.1, payload.0) {
            return Err(InvalidQuorum::InvalidChildBlockError());
        }

        let sk = VVRF::generate_secret_key();
        let mut vvrf = VVRF::new((payload.2).as_bytes(), sk);

        if VVRF::verify_seed(&mut vvrf).is_err() {
            return Err(InvalidQuorum::InvalidSeedError());
        }

        let mut random_number = vvrf.generate_u64();
        while random_number < u32MAX as u64 {
            random_number = vvrf.generate_u64();
        }
        Ok(random_number)
    }

    ///master nodes run elections to determine the next master node quorum
    fn run_election(&mut self, ballot: Self::Ballot) -> Result<&Self::Return, Self::Error> {
        if self.election_block_height == 0 || self.election_timestamp == 0 {
            return Err(InvalidQuorum::InvalidChildBlockError());
        }

        dbg!("one");

        let eligible_claims = match Quorum::get_eligible_claims(ballot) {
            Ok(eligible_claims) => eligible_claims,
            Err(e) => return Err(e),
        };

        dbg!("two");


        let elected_quorum = match self.get_final_quorum(eligible_claims) {
            Ok(elected_quorum) => elected_quorum,
            Err(e) => return Err(e),
        };

        dbg!("three");


        Ok(elected_quorum)
    }
}

impl Quorum {
    ///makes a new Quorum and initializes seed, child block height, and child
    /// block timestamp
    pub fn new(seed: u64, timestamp: u128, height: u128) -> Result<Quorum, InvalidQuorum> {
        if !Quorum::check_payload_validity(height, timestamp) {
            Err(InvalidQuorum::InvalidChildBlockError())
        } else {
            Ok(Quorum {
                quorum_seed: seed,
                master_pubkeys: Vec::new(),
                quorum_pk: String::new(),
                election_block_height: height,
                election_timestamp: timestamp,
            })
        }
    }

    ///checks if the child block height and timestamp are valid
    ///used at seed and quorum creation
    pub fn check_payload_validity(timestamp: Timestamp, height: Height) -> bool {
        height > 0 && timestamp > 0
    }

    ///gets all claims that belong to eligible nodes (master nodes)
    /// needs to be modifed as claim field eligible:  bool needs to become a uX
    /// of staked amt
    pub fn get_eligible_claims(claims: Vec<Claim>) -> Result<Vec<Claim>, InvalidQuorum> {
        let mut eligible_claims = Vec::<Claim>::new();
        claims
            .into_iter()
            .filter(|claim| claim.eligible)
            .for_each(|claim| {
                eligible_claims.push(claim);
            });
        if eligible_claims.len() < 20 {
            return Err(InvalidQuorum::InsufficientNodesError());
        }
        let eligible_claims = eligible_claims;
        Ok(eligible_claims)
    }

    ///gets the final quorum by getting 51% of master nodes with lowest pointer
    /// sums
    pub fn get_final_quorum(&mut self, claims: Vec<Claim>) -> Result<&Quorum, InvalidQuorum> {
        if self.quorum_seed == 0 {
            dbg!("no seed");
            return Err(InvalidQuorum::NoSeedError());
        }

        let num_claims = ((claims.len() as f32) * 0.51).ceil() as usize;

        let mut claim_tuples: Vec<(u128, &String)> = claims
            .iter()
            .filter(|claim| claim.get_pointer(self.quorum_seed as u128).is_some())
            .map(|claim| {
                (
                    claim.get_pointer(self.quorum_seed as u128).unwrap(),
                    &claim.pubkey,
                )
            })
            .collect();

        if claim_tuples.len() < 20 {
            dbg!("invalid pointer sum");
            return Err(InvalidQuorum::InvalidPointerSumError(claims));
        }

        claim_tuples.sort_by_key(|claim_tuple| claim_tuple.0);

        let pubkeys: Vec<String> = claim_tuples
            .into_iter()
            .map(|claim_tuple| claim_tuple.1.clone())
            .take(num_claims)
            .collect();

        let final_pubkeys = Vec::from_iter(pubkeys[0..num_claims].iter().cloned());
        self.master_pubkeys = final_pubkeys;

        Ok(self)
    }
}
