use std::{
    fs::OpenOptions,
    io::{Read, Write},
    path::Path,
};

use hbbft::crypto::{
    serde_impl::SerdeSecret,
    PublicKey as Validator_Pk,
    SecretKey as Validator_Sk,
};
use secp256k1::{PublicKey as Miner_Pk, Secp256k1, SecretKey as Miner_Sk};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct KeyPair {
    pub miner_kp: (Miner_Sk, Miner_Pk),
    pub validator_kp: (Validator_Sk, Validator_Pk),
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
    #[error("Invalid signature ,details : {0}")]
    InvalidSignature(String),
}


impl KeyPair {
    /// Constructs a new, random `Keypair` using thread_rng() which uses RNG
    pub fn random() -> Self {
        let validator_sk: Validator_Sk = Validator_Sk::random();
        let validator_pk: Validator_Pk = validator_sk.public_key();
        let secp = Secp256k1::new();
        let mut rng = rand::thread_rng();
        let (miner_sk, miner_pk) = secp.generate_keypair(&mut rng);
        KeyPair {
            miner_kp: (miner_sk, miner_pk),
            validator_kp: (validator_sk, validator_pk),
        }
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
    pub fn new(validator_sk: Validator_Sk, miner_sk: Miner_Sk) -> Self {
        let secp = Secp256k1::new();
        let pk = Miner_Pk::from_secret_key(&secp, &miner_sk);
        let validator_pk = validator_sk.public_key();
        KeyPair {
            miner_kp: (miner_sk, pk),
            validator_kp: (validator_sk, validator_pk),
        }
    }

    /// Returns this `Keypair` as a byte array
    pub fn from_bytes(
        validator_key_bytes: &[u8],
        miner_key_bytes: &[u8],
    ) -> Result<KeyPair, KeyPairError> {
        let result = bincode::deserialize::<SerdeSecret<Validator_Sk>>(validator_key_bytes);
        let miner_sk = if let Ok(miner_sk) = Miner_Sk::from_slice(miner_key_bytes) {
            miner_sk
        } else {
            return Err(KeyPairError::InvalidKeyPair);
        };

        match result {
            Ok(secret_key) => Ok(KeyPair::new(secret_key.0, miner_sk)),
            Err(e) => Err(KeyPairError::InvalidKeyPair),
        }
    }

    /// Returns this Validator `PublicKey` from byte array `key_bytes`.
    pub fn from_validator_pk_bytes(key_bytes: &[u8]) -> Result<Validator_Pk, KeyPairError> {
        let result = bincode::deserialize::<Validator_Pk>(key_bytes);
        match result {
            Ok(public_key) => Ok(public_key),
            Err(_) => Err(KeyPairError::InvalidPublicKey),
        }
    }

    /// Returns this Miner `PublicKey` from byte array `key_bytes`.
    pub fn from_miner_pk_bytes(key_bytes: &[u8]) -> Result<Miner_Pk, KeyPairError> {
        match Miner_Pk::from_slice(key_bytes) {
            Ok(public_key) => Ok(public_key),
            Err(_) => Err(KeyPairError::InvalidPublicKey),
        }
    }

    /// Returns  Both Validator and Miner `Secret key` as a byte array
    pub fn to_bytes(&self) -> Result<(Vec<u8>, Vec<u8>), KeyPairError> {
        let mut keys = (vec![], vec![]);
        match bincode::serialize(&SerdeSecret(self.validator_kp.0.clone())) {
            Ok(serialized_sk) => {
                keys.0 = serialized_sk;
            },
            Err(e) => {
                return Err(KeyPairError::SerializeKeyError(
                    String::from("validator secret"),
                    e.to_string(),
                ));
            },
        };
        keys.1 = self.miner_kp.0.secret_bytes().to_vec();
        Ok(keys)
    }

    /// Returns this Validator `PublicKey` as a byte array
    pub fn to_validator_pk_bytes(&self) -> Result<Vec<u8>, KeyPairError> {
        match bincode::serialize(&self.validator_kp.1) {
            Ok(serialized_pk) => Ok(serialized_pk),
            Err(e) => Err(KeyPairError::SerializeKeyError(
                String::from("validator public"),
                e.to_string(),
            )),
        }
    }

    /// Returns this Miner `PublicKey` as a byte array
    pub fn to_miner_pk_bytes(&self) -> Result<Vec<u8>, KeyPairError> {
        Ok(self.miner_kp.1.serialize().to_vec())
    }

    /// Gets this `Keypair`'s SecretKey
    pub fn get_secret_key(&self) -> (&Validator_Sk, &Miner_Sk) {
        (&self.validator_kp.0, &self.miner_kp.0)
    }

    pub fn get_public_key(&self) -> (&Validator_Pk, &Miner_Pk) {
        (&self.validator_kp.1, &self.miner_kp.1)
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
            let key_contents: Vec<&str> = contents.split('\n').collect();
            let mut key_bytes: (Vec<u8>, Vec<u8>) = (vec![], vec![]);
            if let Some(validator_sk_content) = key_contents.first() {
                let bytes = get_key_bytes(validator_sk_content);
                if bytes.is_empty() {
                    return Err(KeyPairError::InvalidHex);
                }
                key_bytes.0 = bytes;
            }
            if let Some(miner_sk_content) = key_contents.get(1) {
                let bytes = get_key_bytes(miner_sk_content);
                if bytes.is_empty() {
                    return Err(KeyPairError::InvalidHex);
                }
                key_bytes.1 = bytes;
            }
            let keypair = KeyPair::from_bytes(key_bytes.0.as_slice(), key_bytes.1.as_slice())?;
            Ok(keypair)
        },
        Err(e) => Err(KeyPairError::FailedToReadFromFile(e.to_string())),
    }
}

fn get_key_bytes(miner_sk_content: &&str) -> Vec<u8> {
    let bytes: Vec<u8> = if let Ok(data) = hex::decode(miner_sk_content) {
        return data;
    } else {
        vec![]
    };
    bytes
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
/// A Result<(String,String), KeyPairError>
pub fn write_keypair<W: Write>(
    keypair: &KeyPair,
    writer: &mut W,
) -> Result<(String, String), KeyPairError> {
    let keypair_bytes = keypair.to_bytes()?;
    let serialized_validator_sk = hex::encode(keypair_bytes.0);
    let serialized_miner_sk = hex::encode(keypair_bytes.1);
    let _ = writer.write_all(serialized_validator_sk.as_bytes());
    let _ = writer.write_all("\n".to_string().as_bytes());
    let _ = writer.write_all(serialized_miner_sk.as_bytes());
    Ok((serialized_validator_sk, serialized_miner_sk))
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
/// A Result<(String,String), KeyPairError>
pub fn write_keypair_file<F: AsRef<Path>>(
    keypair: &KeyPair,
    outfile: F,
) -> Result<(String, String), KeyPairError> {
    let outfile = outfile.as_ref();
    if let Some(outdir) = outfile.parent() {
        if let Err(_e) = storage::create_dir(outdir) {
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
    use secp256k1::Message;

    use super::*;

    #[test]
    fn test_serialize_secret_key() {
        let keypair = KeyPair::random();
        let serialized_sk = keypair.to_bytes().unwrap();
        let result = KeyPair::from_bytes(serialized_sk.0.as_slice(), serialized_sk.1.as_slice());
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
            keypair.get_secret_key().0.reveal()
        )
    }


    #[test]
    fn test_write_keypair_file() {
        let outfile = tmp_file_path("test_write_keypair_file.json");
        let keypair = KeyPair::random();
        let serialized_keypair = write_keypair_file(&keypair, &outfile).unwrap();
        let keypair_vec_validator: Vec<u8> = hex::decode(&serialized_keypair.0).unwrap();
        let keypair_vec_miner: Vec<u8> = hex::decode(&serialized_keypair.1).unwrap();
        assert!(Path::new(&outfile).exists());
        let read_keypair = read_keypair_file(&outfile).unwrap().to_bytes().unwrap();
        assert_eq!(keypair_vec_validator, read_keypair.0);
        assert_eq!(keypair_vec_miner, read_keypair.1);

        assert_eq!(
            read_keypair_file(&outfile)
                .unwrap()
                .get_public_key()
                .0
                .to_bytes()
                .to_vec()
                .len(),
            48
        );
        assert_eq!(
            read_keypair_file(&outfile)
                .unwrap()
                .get_public_key()
                .1
                .serialize()
                .len(),
            33
        );
        std::fs::remove_file(&outfile).unwrap();
        assert!(!Path::new(&outfile).exists());

        //Testing signatures
        let msg = "Hello VRRB";
        let deserialized_key =
            KeyPair::from_bytes(read_keypair.0.as_slice(), read_keypair.1.as_slice()).unwrap();

        let validator_sk = deserialized_key.validator_kp.0.clone();
        let validator_pk = deserialized_key.validator_kp.1.clone();
        let miner_sk = deserialized_key.miner_kp.0.clone();
        let miner_pk = deserialized_key.miner_kp.1.clone();

        assert_eq!(keypair.validator_kp.0.sign(msg), validator_sk.sign(msg));
        assert_eq!(validator_pk.verify(&validator_sk.sign(msg), msg), true);
        let validator_pbytes = deserialized_key.to_validator_pk_bytes().unwrap();
        let validator_pkey = KeyPair::from_validator_pk_bytes(&validator_pbytes).unwrap();
        assert_eq!(validator_pkey.verify(&validator_sk.sign(msg), msg), true);

        let secp = Secp256k1::new();
        let msg = Message::from_hashed_data::<secp256k1::hashes::sha256::Hash>(msg.as_bytes());
        assert_eq!(
            secp.sign_ecdsa(&msg, &keypair.miner_kp.0),
            secp.sign_ecdsa(&msg, &miner_sk)
        );

        let sig = secp.sign_ecdsa(&msg, &keypair.miner_kp.0);
        assert!(secp.verify_ecdsa(&msg, &sig, &miner_pk).is_ok());
        let miner_pbytes = deserialized_key.to_miner_pk_bytes().unwrap();
        let miner_pkey = KeyPair::from_miner_pk_bytes(&miner_pbytes).unwrap();
        assert!(secp.verify_ecdsa(&msg, &sig, &miner_pkey).is_ok());
    }


    #[test]
    fn test_write_keypair_file_overwrite_ok() {
        let outfile = tmp_file_path("test_write_keypair_file_overwrite_ok.json");
        write_keypair_file(&KeyPair::random(), &outfile).unwrap();
        write_keypair_file(&KeyPair::random(), &outfile).unwrap();
    }
}
