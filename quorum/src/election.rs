pub trait Election {
    type Return;
    type Error;
    type Ballot;
    type Payload;
 
    fn generate_seed(&mut self, payload: Self::Payload) -> Result<u128, Self::Error>;
    fn elect_quorum(&mut self, ballot: Self::Ballot) -> Result<&Self::Return, Self::Error>;
    fn run_election(&mut self, ballot: Self::Ballot) -> Result<&Self::Return, Self::Error>;
}

