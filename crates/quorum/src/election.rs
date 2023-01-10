use vrrb_core::keypair::KeyPair;

pub trait Election {
    ///generic types for running an election
    type Return;
    type Error;
    type Ballot;
    type Payload;
    type Seed;

    ///generates a seed for the election
    fn generate_seed(payload: Self::Payload, kp: KeyPair) -> Result<Self::Seed, Self::Error>;
    ///runs the election
    fn run_election(&mut self, ballot: Self::Ballot) -> Result<&Self::Return, Self::Error>;
    ///re-make seed and nonce up claims to run a new election in case of
    /// election failure
    fn run_harvester_election() -> Result<Self::Ballot, Self::Error>;
    fn run_farmer_election() -> Result<Self::Ballot, Self::Error>;

    fn nonce_claims_and_new_seed(
        &mut self,
        claims: Self::Ballot,
        kp: KeyPair,
    ) -> Result<Self::Ballot, Self::Error>;
}
