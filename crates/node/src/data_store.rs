use crate::state_reader::StateReader;

#[async_trait::async_trait]
// NOTE: renamed to DataStore to avoid confusion with StateStore within storage crate
pub trait DataStore<S: StateReader> {
    type Error;

    fn state_reader(&self) -> S;
}
