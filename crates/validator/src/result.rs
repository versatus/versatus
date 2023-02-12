pub type Result<T> = std::result::Result<T, ValidatorError>;

#[derive(Debug, thiserror::Error)]
pub enum ValidatorError {
    #[error("validator error: {0}")]
    Other(String),
}
