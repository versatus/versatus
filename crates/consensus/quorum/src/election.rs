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
    fn run_election(&mut self, ballot: Self::Ballot) -> Result<Self::Return, Self::Error>;
}
