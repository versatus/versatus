use std::fmt::Debug;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PlatformError {
    #[error("encountered error while collecting system info")]
    POSIX(#[from] std::io::Error),

    #[error("conversion error: {0}")]
    Conversion(String),
}
impl<'msg, F, I> From<ConversionError<'msg, F, I>> for PlatformError
where
    F: Debug,
    I: Debug,
{
    fn from(value: ConversionError<F, I>) -> Self {
        PlatformError::Conversion(value.to_string())
    }
}

#[derive(Error, Debug, Clone)]
#[error(
    "error occurred while attempting to convert {:?} into {:?}: {}",
    from,
    into,
    msg
)]
pub struct ConversionError<'msg, F, I>
where
    F: Debug,
    I: Debug,
{
    pub(crate) from: F,
    pub(crate) into: I,
    pub(crate) msg: &'msg str,
}
