pub trait Election {
    ///generic types for running an election
    type Return;
    type Error;
    type Ballot;
    type Payload;
    type Seed;

    ///generates a seed for the election
    fn generate_seed(payload: Self::Payload) -> Result<Self::Seed, Self::Error>;
    ///runs the election
    fn run_election(&mut self, ballot: Self::Ballot) -> Result<&Self::Return, Self::Error>;
    ///re-make seed and nonce up claims to run a new election in case of
    /// electoin failure
    fn nonce_claims_and_new_seed(
        &mut self,
        claims: Self::Ballot,
    ) -> Result<Self::Ballot, Self::Error>;
}
