use async_std::io;
use err_derive::Error;

#[derive(Debug, Error)]
pub enum DiscovererError {

        #[error(display = "Failed to deserialize NodeRouteEntry from bytes: {}", _0)]
        Deserialize(String),

            #[error(display = "IO Error: {}", _0)]
            Io(io::Error),
}

#[derive(PartialEq, Eq, Debug)]
pub enum NodePoolError {
    NodeMissing,
}

