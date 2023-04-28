use std::collections::BTreeMap;

use ethereum_types::U256;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use vrrb_core::{
    claim::{Claim, Eligibility},
    keypair::KeyPair,
};
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

    #[error("none values from claim")]
    ClaimError(),
}

///Quorum struct which is created and modified when an election is run
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct Quorum {
    pub quorum_seed: u64,
    pub master_pubkeys: Vec<String>,
    pub quorum_pk: String,
    pub election_block_height: u128,
}

///generic types from Election trait defined here for Quorums
type Height = u128;
type BlockHash = String;
type Seed = u64;

///Payload data comes from current child block
impl Election for Quorum {
    type Ballot = Vec<Claim>;
    type Error = InvalidQuorum;
    type Payload = (Height, BlockHash);
    type Return = Self;
    type Seed = Seed;

    ///a miner calls this fxn to generate a u64 seed for the election using the
    /// vrrb_vrf crate
    fn generate_seed(payload: Self::Payload, kp: KeyPair) -> Result<Seed, InvalidQuorum> {
        if !Quorum::check_validity(payload.0) {
            return Err(InvalidQuorum::InvalidChildBlockError());
        }
        let mut vvrf = VVRF::new(
            (payload.1).as_bytes(),
            kp.miner_kp.0.secret_bytes().as_slice(),
        );

        if VVRF::verify_seed(&mut vvrf).is_err() {
            return Err(InvalidQuorum::InvalidSeedError());
        }

        let mut random_number = vvrf.generate_u64();
        while random_number < u32::MAX as u64 {
            random_number = vvrf.generate_u64();
        }
        Ok(random_number)
    }

    ///master nodes run elections to determine the next master node quorum
    fn run_election(&mut self, ballot: Self::Ballot) -> Result<&Self::Return, Self::Error> {
        if self.election_block_height == 0 {
            return Err(InvalidQuorum::InvalidChildBlockError());
        }

        let eligible_claims = match Quorum::get_eligible_claims(ballot) {
            Ok(eligible_claims) => eligible_claims,
            Err(e) => return Err(e),
        };

        let elected_quorum = match self.get_final_quorum(eligible_claims) {
            Ok(elected_quorum) => elected_quorum,
            Err(e) => return Err(e),
        };

        Ok(elected_quorum)
    }
}

impl Quorum {
    ///makes a new Quorum and initializes seed, child block height, and child
    /// block timestamp
    pub fn new(seed: u64, height: u128) -> Result<Quorum, InvalidQuorum> {
        if !Quorum::check_validity(height) {
            Err(InvalidQuorum::InvalidChildBlockError())
        } else {
            Ok(Quorum {
                quorum_seed: seed,
                master_pubkeys: Vec::new(),
                quorum_pk: String::new(),
                election_block_height: height,
            })
        }
    }

    ///checks if the child block height is valid ,its used at seed and quorum
    /// creation
    pub fn check_validity(height: Height) -> bool {
        height > 0
    }

    ///gets all claims that belong to eligible nodes (master nodes)
    /// needs to be modifed as claim field eligible:  bool needs to become a uX
    /// of staked amt
    pub fn get_eligible_claims(claims: Vec<Claim>) -> Result<Vec<Claim>, InvalidQuorum> {
        let mut eligible_claims = Vec::<Claim>::new();
        claims
            .into_iter()
            .filter(|claim| {
                claim.eligibility == Eligibility::Harvester
                    && claim.eligibility == Eligibility::Farmer
            })
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
            return Err(InvalidQuorum::NoSeedError());
        }

        let num_claims = ((claims.len() as f32) * 0.51).ceil() as usize;

        let election_results: BTreeMap<U256, Claim> = claims
            .iter()
            .map(|claim| {
                (
                    claim.get_election_result(self.quorum_seed.clone()),
                    claim.clone(),
                )
            })
            .collect();

        if election_results.len() < (((claims.len() as f32) * 0.65).ceil() as usize) {
            return Err(InvalidQuorum::InvalidPointerSumError(claims));
        }

        let pubkeys: Vec<String> = election_results
            .iter()
            .map(|(_, claim)| claim.public_key.clone().to_string())
            .take(num_claims)
            .collect();

        let final_pubkeys = Vec::from_iter(pubkeys[0..num_claims].iter().cloned());
        self.master_pubkeys = final_pubkeys;

        Ok(self)
    }

    pub fn get_trusted_peers(&mut self, _claims: Vec<Claim>) -> Self {
        //get the weighted value hashmap
        //calcualte all the t values

        todo!();
    }
}
