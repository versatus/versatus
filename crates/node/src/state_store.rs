use crate::{state_reader::StateReader, state_writer::StateWriter};

#[async_trait::async_trait]
pub trait StateStore {
    type Error;
    async fn write();
    fn read();
}
