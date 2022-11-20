#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = anyhow::Result<T>;
