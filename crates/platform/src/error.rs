use std::fmt::Debug;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PlatformError {
    #[error("encountered error while collecting system info")]
    POSIX(#[from] std::io::Error),

    #[error("conversion error: {0}")]
    Conversion(String),
}
impl<'msg, A, B> From<ConversionError<'msg, A, B>> for PlatformError
where
    A: Debug,
    B: Debug,
{
    fn from(value: ConversionError<A, B>) -> Self {
        PlatformError::Conversion(value.to_string())
    }
}

#[derive(Error, Debug)]
#[error(
    "error occurred while attempting to convert {:?} into {:?}: {}",
    from,
    into,
    msg
)]
pub struct ConversionError<'msg, A, B>
where
    A: Debug,
    B: Debug,
{
    pub(crate) from: A,
    pub(crate) into: B,
    pub(crate) msg: &'msg str,
}
