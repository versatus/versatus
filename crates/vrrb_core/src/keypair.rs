use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

use hbbft::crypto::{serde_impl::SerdeSecret, PublicKey, SecretKey};
use thiserror::Error;

#[derive(Debug)]
pub struct KeyPair {
    pub sk: SecretKey,
    pub pk: PublicKey,
}


#[derive(Error, Debug)]
pub enum KeyPairError {
    #[error("Failed to deserialize the secret key from bytes")]
    InvalidKeyPair,
    #[error("Failed to serialize the  key to bytes, details :{0}")]
    SerializeKeyError(String, String),
    #[error("Failed to read key from file ,details :{0}")]
    FailedToReadFromFile(String),
    #[error("Invalid Hex represenation of secret key")]
    InvalidHex,
    #[error("Failed to create directory for storage of  secret key :{0}")]
    IOError(String),
    #[error("Failed to deserialize the public key from bytes")]
    InvalidPublicKey,
}


impl KeyPair {
    /// Constructs a new, random `Keypair` using thread_rng() which uses RNG
    pub fn random() -> Self {
        let sk = SecretKey::random();
        let pk = sk.public_key();
        KeyPair { sk, pk }
    }

    /// `new` takes a `SecretKey` and returns a `KeyPair`
    ///
    /// Arguments:
    ///
    /// * `sk`: SecretKey
    ///
    /// Returns:
    ///
    /// A KeyPair struct
    pub fn new(sk: SecretKey) -> Self {
        KeyPair {
            sk: sk.clone(),
            pk: sk.public_key(),
        }
    }

    /// Returns this `Keypair` as a byte array
    pub fn from_bytes(key_bytes: &[u8]) -> Result<KeyPair, KeyPairError> {
        let result = bincode::deserialize::<SerdeSecret<SecretKey>>(key_bytes);
        match result {
            Ok(secret_key) => Ok(KeyPair::new(secret_key.0)),
            Err(_) => Err(KeyPairError::InvalidKeyPair),
        }
    }

    /// Returns this `PublicKey` from byte array `key_bytes`.
    pub fn from_pbytes(key_bytes: &[u8]) -> Result<PublicKey, KeyPairError> {
        let result = bincode::deserialize::<PublicKey>(key_bytes);
        match result {
            Ok(public_key) => Ok(public_key),
            Err(_) => Err(KeyPairError::InvalidPublicKey),
        }
    }

    /// Returns this `Secret key` as a byte array
    pub fn to_bytes(&self) -> Result<Vec<u8>, KeyPairError> {
        match bincode::serialize(&SerdeSecret(self.sk.clone())) {
            Ok(serialized_sk) => Ok(serialized_sk),
            Err(e) => Err(KeyPairError::SerializeKeyError(
                String::from("secret"),
                e.to_string(),
            )),
        }
    }

    /// Returns this `Secret key` as a byte array
    pub fn to_pbytes(&self) -> Result<Vec<u8>, KeyPairError> {
        match bincode::serialize(&self.pk) {
            Ok(serialized_pk) => Ok(serialized_pk),
            Err(e) => Err(KeyPairError::SerializeKeyError(
                String::from("public"),
                e.to_string(),
            )),
        }
    }

    /// Gets this `Keypair`'s SecretKey
    pub fn get_secret_key(&self) -> &SecretKey {
        &self.sk
    }

    pub fn get_public_key(&self) -> &PublicKey {
        &self.pk
    }
}


/// Reads a Hex-encoded `Keypair` from a `Reader` implementor
/// It reads a file, decodes the hex, and then creates a KeyPair from the bytes
///
/// Arguments:
///
/// * `reader`: &mut R - This is the file handle that we'll read the keypair
///   from.
///
/// Returns:
///
/// A Result<KeyPair, KeyPairError>
pub fn read_keypair<R: Read>(reader: &mut R) -> Result<KeyPair, KeyPairError> {
    let mut contents = String::new();
    match reader.read_to_string(&mut contents) {
        Ok(_) => {
            let bytes: Vec<u8> = if let Ok(data) = hex::decode(contents) {
                data
            } else {
                return Err(KeyPairError::InvalidHex);
            };
            let keypair = KeyPair::from_bytes(bytes.as_slice())?;
            Ok(keypair)
        },
        Err(e) => Err(KeyPairError::FailedToReadFromFile(e.to_string())),
    }
}

/// Reads a `Keypair` from a file
/// It opens a file, reads the contents of the file, and then parses the
/// contents of the file as a keypair
///
/// Arguments:
///
/// * `path`: The path to the file to read from.
///
/// Returns:
///
/// A Result<KeyPair, KeyPairError>
pub fn read_keypair_file<F: AsRef<Path>>(path: F) -> Result<KeyPair, KeyPairError> {
    match storage::read_file(path.as_ref()) {
        Ok(mut file) => read_keypair(&mut file),
        Err(e) => Err(KeyPairError::FailedToReadFromFile(e.to_string())),
    }
}


/// Writes a `Keypair` to a `Write` implementor with HEX-encoding
/// It takes a `KeyPair` and a `Write`r, and writes the serialized `KeyPair` to
/// the `Write`r
///
/// Arguments:
///
/// * `keypair`: The keypair to write to the file.
/// * `writer`: The writer to write the keypair to.
///
/// Returns:
///
/// A Result<String, KeyPairError>
pub fn write_keypair<W: Write>(keypair: &KeyPair, writer: &mut W) -> Result<String, KeyPairError> {
    let keypair_bytes = keypair.to_bytes()?;
    let serialized = hex::encode(keypair_bytes);
    let _ = writer.write_all(serialized.as_bytes());
    Ok(serialized)
}

/// Writes a `Keypair` to a file with HEX-encoding
/// It writes the keypair to a file.
///
/// Arguments:
///
/// * `keypair`: &KeyPair,
/// * `outfile`: The path to the file where the keypair will be stored.
///
/// Returns:
///
/// A Result<String, KeyPairError>
pub fn write_keypair_file<F: AsRef<Path>>(
    keypair: &KeyPair,
    outfile: F,
) -> Result<String, KeyPairError> {
    let outfile = outfile.as_ref();
    if let Some(outdir) = outfile.parent() {
        if let Err(e) = storage::create_dir(outdir) {
            return Err(KeyPairError::IOError(
                "Failed to create directory for storage of  secret key".to_string(),
            ));
        };
    }

    match {
        #[cfg(not(unix))]
        {
            OpenOptions::new()
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            OpenOptions::new().mode(0o600)
        }
    }
    .write(true)
    .truncate(true)
    .create(true)
    .open(outfile)
    {
        Ok(mut f) => write_keypair(keypair, &mut f),
        Err(_) => Err(KeyPairError::IOError(
            "Failed to open directory for storage of  secret key".to_string(),
        )),
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_secret_key() {
        let keypair = KeyPair::random();
        let serialized_sk = keypair.to_bytes().unwrap();
        let result = KeyPair::from_bytes(serialized_sk.as_slice());
        assert!(result.is_ok());
    }


    /// It creates a temporary file path for a given name, and returns it as a
    /// string
    ///
    /// Arguments:
    ///
    /// * `name`: The name of the file.
    ///
    /// Returns:
    ///
    /// A string
    fn tmp_file_path(name: &str) -> String {
        use std::env;
        let out_dir = env::var("TMP_DIR").unwrap_or_else(|_| "tmp_key".to_string());
        let keypair = KeyPair::random();
        format!(
            "{}/tmp/{}-{}",
            out_dir,
            name,
            keypair.get_secret_key().reveal()
        )
    }


    #[test]
    fn test_write_keypair_file() {
        let outfile = tmp_file_path("test_write_keypair_file.json");
        let keypair = KeyPair::random();
        let serialized_keypair = write_keypair_file(&keypair, &outfile).unwrap();
        let keypair_vec: Vec<u8> = hex::decode(&serialized_keypair).unwrap();
        assert!(Path::new(&outfile).exists());
        assert_eq!(
            keypair_vec,
            read_keypair_file(&outfile).unwrap().to_bytes().unwrap()
        );
        assert_eq!(
            read_keypair_file(&outfile)
                .unwrap()
                .get_public_key()
                .to_bytes()
                .to_vec()
                .len(),
            48
        );
        fs::remove_file(&outfile).unwrap();
        assert!(!Path::new(&outfile).exists());

        //Testing signatures
        let msg = "Hello VRRB";
        let sk = keypair.get_secret_key();

        let deserialized_key = KeyPair::from_bytes(keypair_vec.as_slice()).unwrap();
        assert_eq!(sk.sign(msg), deserialized_key.get_secret_key().sign(msg));
        assert!(deserialized_key.get_public_key().verify(&sk.sign(msg), msg) == true);
        let pbytes = deserialized_key.to_pbytes().unwrap();
        let pkey = KeyPair::from_pbytes(&pbytes).unwrap();
        assert!(pkey.verify(&sk.sign(msg), msg) == true);
    }


    #[test]
    fn test_write_keypair_file_overwrite_ok() {
        let outfile = tmp_file_path("test_write_keypair_file_overwrite_ok.json");
        write_keypair_file(&KeyPair::random(), &outfile).unwrap();
        write_keypair_file(&KeyPair::random(), &outfile).unwrap();
    }
}
