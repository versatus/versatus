pub type Result<T> = std::result::Result<T, ConfigError>;

#[derive(Debug, Clone, thiserror::Error)]
pub enum ConfigError {
    #[error("{0}")]
    Other(String),
}
