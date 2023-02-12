use primitives::{ByteSlice, ByteVec};
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

pub fn encode_to_json<T: Serialize>(data: &T) -> Result<ByteVec> {
    serde_json::to_vec(data).map_err(|err| Error::Other(err.to_string()))
}

pub fn encode_to_binary<T: Serialize>(data: &T) -> Result<ByteVec> {
    bincode::serialize(data).map_err(|err| Error::Other(err.to_string()))
}

pub fn decode_bytes<T>(data: ByteSlice) -> Result<T>
where
    T: for<'a> Deserialize<'a> + Default,
{
    if let Ok(result) = decode_from_json_byte_slice::<T>(data) {
        return Ok(result);
    }

    if let Ok(result) = decode_from_binary_byte_slice::<T>(data) {
        return Ok(result);
    }

    Ok(T::default())
}

pub fn decode_from_json_byte_slice<T>(data: ByteSlice) -> Result<T>
where
    T: for<'a> Deserialize<'a>,
{
    serde_json::from_slice::<T>(data).map_err(|err| Error::Other(err.to_string()))
}

pub fn decode_from_binary_byte_slice<T>(data: ByteSlice) -> Result<T>
where
    T: for<'a> Deserialize<'a>,
{
    bincode::deserialize::<T>(data).map_err(|err| Error::Other(err.to_string()))
}
