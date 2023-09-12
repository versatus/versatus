use std::{io::Read, path::Path};

use primitives::{PublicKey, SecretKey};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use ritelinked::LinkedHashMap;
use secp256k1::generate_keypair;

use crate::{
    transactions::transfer::Transfer,
    transactions::transaction::TransactionDigest,
    Error,
};

pub fn gen_hex_encoded_string<T: AsRef<[u8]>>(data: T) -> String {
    hex::encode(data)
}

// NOTE: this is used to generate random filenames so files created by tests
// don't get overwritten
pub fn generate_random_string() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(30)
        .map(char::from)
        .collect()
}

#[macro_export]
macro_rules! is_enum_variant {
    ($v:expr, $p:pat) => {
        if let $p = $v {
            true
        } else {
            false
        }
    };
}

pub fn size_of_txn_list(txns: &LinkedHashMap<TransactionDigest, Transfer>) -> usize {
    txns.iter()
        .map(|(_, set)| set)
        .map(std::mem::size_of_val)
        .sum()
}

fn read_keypair_file<F: AsRef<Path>>(path: F) -> crate::Result<(SecretKey, PublicKey)> {
    match crate::storage_utils::read_file(path.as_ref()) {
        Ok(mut file) => read_keypair(&mut file),
        Err(e) => Err(Error::Other(e.to_string())),
    }
}

pub fn read_or_generate_keypair_file<F: AsRef<Path>>(
    path: F,
) -> crate::Result<(SecretKey, PublicKey)> {
    let keypair = match read_keypair_file(&path) {
        Ok(keypair) => keypair,
        Err(err) => {
            telemetry::warn!("Failed to read keypair file: {}", err);
            telemetry::info!("Generating new keypair");

            let keypair = generate_keypair(&mut rand::thread_rng());

            write_keypair_file(&path, &keypair).map_err(|err| {
                crate::Error::Other(format!("failed to write keypair file: {err}"))
            })?;

            keypair
        },
    };

    Ok(keypair)
}

pub fn write_keypair_file<F: AsRef<Path>>(
    path: F,
    keypair: &(SecretKey, PublicKey),
) -> crate::Result<()> {
    let (secret_key, public_key) = keypair;

    let sk_ser = bincode::serialize(secret_key)
        .map_err(|err| crate::Error::Other(format!("failed to serialize secret key: {err}")))?;

    let pk_ser = bincode::serialize(public_key)
        .map_err(|err| crate::Error::Other(format!("failed to serialize public key: {err}")))?;

    // TODO: store keypairs securely
    let contents = format!("{}\n{}", hex::encode(sk_ser), hex::encode(pk_ser));

    std::fs::write(path, contents)
        .map_err(|err| crate::Error::Other(format!("failed to write keypair file: {err}")))?;

    Ok(())
}

fn read_keypair<R: Read>(reader: &mut R) -> crate::Result<(SecretKey, PublicKey)> {
    let mut contents = String::new();

    reader.read_to_string(&mut contents)?;

    let key_contents: Vec<&str> = contents.split('\n').collect();

    let decoded_sk = hex::decode(key_contents[0])
        .map_err(|err| crate::Error::Other(format!("failed to decode secret key: {err}")))?;

    let decoded_pk = hex::decode(key_contents[1])
        .map_err(|err| crate::Error::Other(format!("failed to decode public key: {err}")))?;

    let deserialized_sk = bincode::deserialize::<SecretKey>(&decoded_sk)
        .map_err(|err| crate::Error::Other(format!("failed to deserialize secret key: {err}")))?;

    let deserialized_pk = bincode::deserialize::<PublicKey>(&decoded_pk)
        .map_err(|err| crate::Error::Other(format!("failed to deserialize public key: {err}")))?;

    Ok((deserialized_sk, deserialized_pk))
}
