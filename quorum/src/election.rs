pub trait Election {
    type Return;
    type Error;
    type Ballot;
    type Payload;

    fn elect_quorum(
        &mut self,
        payload: Self::Payload,
        ballot: Self::Ballot,
    ) -> Result<&Self::Return, Self::Error>;
    fn run_election(
        &mut self,
        payload: Self::Payload,
        ballot: Self::Ballot,
    ) -> Result<&Self::Return, Self::Error>;
}
