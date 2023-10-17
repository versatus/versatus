use std::{
    fs::OpenOptions,
    hash::{Hash, Hasher},
    io::{Read, Write},
    path::Path,
    str::FromStr,
};

use bs58::encode;
use hbbft::crypto::serde_impl::SerdeSecret;
use primitives::{PublicKey, SecretKey, SerializedSecretKey as SecretKeyBytes};
use ring::digest::{Context, SHA256};
use secp256k1::{ecdsa::Signature, Message, Secp256k1};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::storage_utils;

#[deprecated(note = "use MinerSecretKey instead")]
pub type MinerSk = secp256k1::SecretKey;

#[deprecated(note = "use MinerPublicKey instead")]
pub type MinerPk = secp256k1::PublicKey;

pub type MinerPublicKey = secp256k1::PublicKey;
pub type MinerSecretKey = secp256k1::SecretKey;

pub type SecretKeys = (MinerSecretKey, SecretKey);
pub type PublicKeys = (MinerPublicKey, PublicKey);

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct KeyPair {
    pub miner_kp: (MinerSk, MinerPk),
    pub validator_kp: (SecretKey, PublicKey),
}

/// Alias for KeyPair, to avoid frustrations because of subtle typos
pub type Keypair = KeyPair;

#[derive(Error, Debug)]
pub enum KeyPairError {
    #[error("Failed to deserialize the secret key from bytes")]
    InvalidKeyPair,
    #[error("Failed to serialize the  key to bytes: {0}")]
    SerializeKeyError(String, String),
    #[error("Failed to read key from file: {0}")]
    FailedToReadFromFile(String),
    #[error("Invalid Hex represenation of secret key")]
    InvalidHex,
    #[error("Failed to create directory for storage of secret key: {0}")]
    IOError(String),
    #[error("Failed to deserialize the public key from bytes")]
    InvalidPublicKey,
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),
    #[error("ECDSA Signature Verification failed: {0}")]
    SignatureVerificationFailed(String),
    #[error("Failed to deserialize {0} key ")]
    InvalidKey(String),
}

pub type Result<T> = std::result::Result<T, KeyPairError>;

impl KeyPair {
    /// Constructs a new, random `Keypair` using thread_rng() which uses RNG
    pub fn random() -> Self {
        let secp = Secp256k1::new();
        let mut rng = rand::thread_rng();
        let (sk, pk) = secp.generate_keypair(&mut rng);
        KeyPair {
            // TODO: Consider renaming to simply sk and pk as this pair is not only used for mining
            miner_kp: (sk.clone(), pk.clone()),
            validator_kp: (sk, pk),
        }
    }

    /// > The peer ID is the first 20 bytes of the SHA256 hash of the miner's
    /// > public key, encoded using
    /// base58
    ///
    /// Returns:
    ///
    /// The peer id is being returned.
    pub fn get_peer_id(&self) -> String {
        let miner_public_key = self.get_miner_public_key();

        let mut context = Context::new(&SHA256);
        context.update(miner_public_key.serialize().as_slice());
        let hash = context.finish();

        // Take the first 20 bytes of the hash
        let peer_id = &hash.as_ref()[..20];

        // Encode the peer ID using base58
        encode(peer_id).into_string()
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
    pub fn new(sk: SecretKey, _miner_sk: MinerSk) -> Self {
        let secp = Secp256k1::new();
        let pk = sk.public_key(&secp);
        KeyPair {
            miner_kp: (sk.clone(), pk.clone()),
            validator_kp: (sk, pk),
        }
    }

    /// Returns this `Keypair` as a byte array
    pub fn from_bytes(validator_key_bytes: &[u8], miner_key_bytes: &[u8]) -> Result<KeyPair> {
        let result = bincode::deserialize::<SerdeSecret<SecretKey>>(validator_key_bytes);
        let miner_sk = if let Ok(miner_sk) = MinerSk::from_slice(miner_key_bytes) {
            miner_sk
        } else {
            return Err(KeyPairError::InvalidKeyPair);
        };

        match result {
            Ok(secret_key) => Ok(KeyPair::new(secret_key.0, miner_sk)),
            Err(_) => Err(KeyPairError::InvalidKeyPair),
        }
    }

    /// Returns this Validator `PublicKey` from byte array `key_bytes`.
    pub fn from_validator_pk_bytes(key_bytes: &[u8]) -> Result<PublicKey> {
        let result = bincode::deserialize::<PublicKey>(key_bytes);
        match result {
            Ok(public_key) => Ok(public_key),
            Err(_) => Err(KeyPairError::InvalidPublicKey),
        }
    }

    /// Returns this Miner `PublicKey` from byte array `key_bytes`.
    pub fn from_miner_pk_bytes(key_bytes: &[u8]) -> Result<MinerPk> {
        match MinerPk::from_slice(key_bytes) {
            Ok(public_key) => Ok(public_key),
            Err(_) => Err(KeyPairError::InvalidPublicKey),
        }
    }

    /// Returns  Both Validator and Miner `Secret key` as a byte array
    pub fn to_bytes(&self) -> Result<(Vec<u8>, Vec<u8>)> {
        let mut keys = (vec![], vec![]);
        keys.0 = self.validator_kp.0.secret_bytes().to_vec();
        keys.1 = self.miner_kp.0.secret_bytes().to_vec();
        Ok(keys)
    }

    /// Returns this Validator `PublicKey` as a byte array
    // TODO: Remove Result here
    pub fn to_validator_pk_bytes(&self) -> Result<Vec<u8>> {
        Ok(self.get_validator_public_key().serialize().to_vec())
    }

    /// Returns this Miner `PublicKey` as a byte array
    pub fn to_miner_pk_bytes(&self) -> Result<Vec<u8>> {
        Ok(self.get_miner_public_key().serialize().to_vec())
    }

    /// Gets this `Keypair`'s SecretKey
    pub fn get_secret_keys(&self) -> (&SecretKey, &MinerSk) {
        (&self.validator_kp.0, &self.miner_kp.0)
    }

    /// > This function returns a tuple of references to the public keys of the
    /// > validator and miner
    pub fn get_public_keys(&self) -> (&PublicKey, &MinerPk) {
        (&self.validator_kp.1, &self.miner_kp.1)
    }

    /// > It takes a message and a secret key, and returns a signature
    ///
    /// Arguments:
    ///
    /// * `msg`: The message to sign.
    /// * `secret_key`: The secret key of the account that will sign the
    ///   message.
    ///
    /// Returns:
    ///
    /// A string of the signature
    pub fn ecdsa_sign(msg: &[u8], secret_key: SecretKeyBytes) -> Result<String> {
        let secp = Secp256k1::new();
        let msg = Message::from_hashed_data::<secp256k1::hashes::sha256::Hash>(msg);
        if let Ok(sk) = SecretKey::from_slice(secret_key.as_slice()) {
            Ok(secp.sign_ecdsa(&msg, &sk).to_string())
        } else {
            Err(KeyPairError::InvalidKey(String::from("Secret")))
        }
    }

    /// > This function takes a signature, a message, and a public key, and
    /// > returns a boolean indicating
    /// whether the signature is valid for the message and public key
    ///
    /// Arguments:
    ///
    /// * `signature`: The signature to verify
    /// * `msg`: The message to be signed
    /// * `pub_key`: The public key of the signer.
    ///
    /// Returns:
    ///
    /// A Result<(), KeyPairError>
    pub fn verify_ecdsa_sign(signature: String, msg: &[u8], pub_key: Vec<u8>) -> Result<()> {
        if let Ok(pk) = secp256k1::PublicKey::from_slice(pub_key.as_slice()) {
            let secp = Secp256k1::new();
            let msg = Message::from_hashed_data::<secp256k1::hashes::sha256::Hash>(msg);
            let sig_result = Signature::from_str(signature.as_str());
            match sig_result {
                Ok(sig) => {
                    let status = secp.verify_ecdsa(&msg, &sig, &pk);
                    match status {
                        Ok(status) => Ok(status),
                        Err(e) => Err(KeyPairError::SignatureVerificationFailed(e.to_string())),
                    }
                },
                Err(e) => Err(KeyPairError::InvalidSignature(e.to_string())),
            }
        } else {
            Err(KeyPairError::InvalidKey(String::from("Public")))
        }
    }

    /// > This function takes a secret key and returns a public key
    ///
    /// Arguments:
    ///
    /// * `key`: MinerSk - The secret key of the miner
    ///
    /// Returns:
    ///
    /// The public key of the miner.
    pub fn get_miner_public_key_from_secret_key(key: MinerSk) -> MinerPk {
        let secp = Secp256k1::new();
        MinerPk::from_secret_key(&secp, &key)
    }

    /// It returns the miner secret key.
    ///
    /// Returns:
    ///
    /// The miner secret key.
    pub fn get_miner_secret_key(&self) -> &MinerSk {
        &self.miner_kp.0
    }

    /// It returns the public key of the miner.
    ///
    /// Returns:
    ///
    /// The public key of the miner.
    pub fn get_miner_public_key(&self) -> &MinerPk {
        &self.miner_kp.1
    }

    /// > This function returns the secret key of the validator
    ///
    /// Returns:
    ///
    /// The validator secret key.
    pub fn get_validator_secret_key(&self) -> &SecretKey {
        &self.validator_kp.0
    }

    pub fn get_validator_secret_key_owned(&self) -> SecretKey {
        self.validator_kp.0.clone()
    }

    /// It returns the public key of the validator.
    ///
    /// Returns:
    ///
    /// The public key of the validator.
    pub fn get_validator_public_key(&self) -> &PublicKey {
        &self.validator_kp.1
    }

    pub fn validator_public_key_owned(&self) -> PublicKey {
        self.validator_kp.1
    }

    #[deprecated(note = "use validator_public_key_owned instead")]
    pub fn get_validator_public_key_owned(&self) -> PublicKey {
        self.validator_public_key_owned()
    }

    pub fn miner_public_key_owned(&self) -> MinerPublicKey {
        self.miner_kp.1
    }

    pub fn miner_secret_key_owned(&self) -> MinerSecretKey {
        self.miner_kp.0
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
pub fn read_keypair<R: Read>(reader: &mut R) -> Result<KeyPair> {
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
pub fn read_keypair_file<F: AsRef<Path>>(path: F) -> Result<KeyPair> {
    match crate::storage_utils::read_file(path.as_ref()) {
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
pub fn write_keypair<W: Write>(keypair: &KeyPair, writer: &mut W) -> Result<(String, String)> {
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
) -> Result<(String, String)> {
    let outfile = outfile.as_ref();
    if let Some(outdir) = outfile.parent() {
        if let Err(_e) = storage_utils::create_dir(outdir) {
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

// impl Serialize for KeyPair {
//     fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         let mut s = serializer.serialize_struct("KeyPair", 2)?;
//         s.serialize_field("miner_kp", &self.miner_kp)?;

//         let wrapped_validator_sk = SerdeSecret(&self.validator_kp.0);
//         let validator_kp_serializable = (&wrapped_validator_sk, &self.validator_kp.1);
//         s.serialize_field("validator_kp", &validator_kp_serializable)?;

//         s.end()
//     }
// }

#[allow(clippy::derived_hash_with_manual_eq)]
impl Hash for KeyPair {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash miner_kp
        let miner_sk_serialized = serde_json::to_string(&self.miner_kp.0).unwrap();
        miner_sk_serialized.hash(state);

        let miner_pk_serialized = serde_json::to_string(&self.miner_kp.1).unwrap();
        miner_pk_serialized.hash(state);

        // Hash validator_kp
        let validator_sk_serialized = serde_json::to_string(&self.validator_kp.0).unwrap();
        validator_sk_serialized.hash(state);

        let validator_pk_serialized = serde_json::to_string(&self.validator_kp.1).unwrap();
        validator_pk_serialized.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use sha2::Digest;

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
            keypair.get_secret_keys().0.display_secret()
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
                .get_public_keys()
                .0
                .serialize()
                .len(),
            33
        );
        assert_eq!(
            read_keypair_file(&outfile)
                .unwrap()
                .get_public_keys()
                .1
                .serialize()
                .len(),
            33
        );
        std::fs::remove_file(&outfile).unwrap();
        assert!(!Path::new(&outfile).exists());

        //Testing signatures
        let mut hasher = sha2::Sha256::new();
        let msg = b"Hello VRRB";
        hasher.update(msg);
        let res = hasher.finalize();
        let msg = secp256k1::Message::from_slice(&res).unwrap();
        let deserialized_key =
            KeyPair::from_bytes(read_keypair.0.as_slice(), read_keypair.1.as_slice()).unwrap();

        let validator_sk = deserialized_key.get_validator_secret_key();
        let validator_pk = deserialized_key.get_validator_public_key();
        let miner_sk = deserialized_key.get_miner_secret_key();
        let miner_pk = deserialized_key.get_miner_public_key();

        assert_eq!(
            keypair.validator_kp.0.sign_ecdsa(msg),
            validator_sk.sign_ecdsa(msg)
        );
        assert!(&validator_sk
            .sign_ecdsa(msg)
            .verify(&msg, &validator_pk)
            .is_ok());
        let validator_pbytes = deserialized_key.to_validator_pk_bytes().unwrap();
        let validator_pkey = KeyPair::from_validator_pk_bytes(&validator_pbytes).unwrap();
        assert!(&validator_sk
            .sign_ecdsa(msg)
            .verify(&msg, &validator_pkey)
            .is_ok());

        let secp = Secp256k1::new();
        assert_eq!(
            secp.sign_ecdsa(&msg, &keypair.miner_kp.0),
            secp.sign_ecdsa(&msg, miner_sk)
        );

        let sig = secp.sign_ecdsa(&msg, &keypair.miner_kp.0);
        assert!(secp.verify_ecdsa(&msg, &sig, miner_pk).is_ok());
        let miner_pbytes = deserialized_key.to_miner_pk_bytes().unwrap();
        let miner_pkey = KeyPair::from_miner_pk_bytes(&miner_pbytes).unwrap();
        assert!(secp.verify_ecdsa(&msg, &sig, &miner_pkey).is_ok());
        let sig = KeyPair::ecdsa_sign(
            "Hello VRRB".as_bytes(),
            keypair.miner_kp.0.secret_bytes().to_vec(),
        );
        let status = KeyPair::verify_ecdsa_sign(
            sig.unwrap(),
            "Hello VRRB".as_bytes(),
            keypair.miner_kp.1.serialize().to_vec(),
        );
        assert!(status.is_ok());
    }

    #[test]
    fn test_write_keypair_file_overwrite_ok() {
        let outfile = tmp_file_path("test_write_keypair_file_overwrite_ok.json");
        write_keypair_file(&KeyPair::random(), &outfile).unwrap();
        write_keypair_file(&KeyPair::random(), &outfile).unwrap();
    }
}
