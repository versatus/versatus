pub trait Election {
    type Return;
    type Error;
    type Ballot;
    type Payload;
    type Seed;
 
    fn generate_seed(payload: Self::Payload) -> Result<Self::Seed, Self::Error>;
    fn run_election(&mut self, ballot: Self::Ballot) -> Result<&Self::Return, Self::Error>;
}

