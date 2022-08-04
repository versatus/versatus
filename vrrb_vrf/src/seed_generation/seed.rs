use vrf::openssl::{CipherSuite, ECVRF};
use vrf::VRF
use secp256k1::{rand, Secp256k1, SecretKey};

fn generate_secret_key() -> SecretKey {
    let secp = Secp256k1::new();
    let secret_key = SecretKey::new(&mut rand::thread_rng());
    secret_key
}

pub fn generate_seed() -> Vec<u8> {
    let secret_key = generate_secret_key();
    let vrf_secret_key = hex::decode(secret_key).unwrap();

    let public_key = vrf.derive_public_key(&secret_key).unwrap();
    let message: &[u8] = b"sample";
    let pi = vrf.prove(&secret_key, &message).unwrap();
    pi
}


