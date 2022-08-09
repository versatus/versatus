use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use secp256k1::{rand::thread_rng, Secp256k1, SecretKey};
use vrf::openssl::{CipherSuite, ECVRF};
use vrf::VRF;
use rand::prelude::*;

fn generate_secret_key() -> SecretKey {
    let secp = Secp256k1::new();
    let secret_key = SecretKey::new(&mut rand::thread_rng());

    secret_key
}

fn generate_seed() -> Vec<u8> {
    let mut vrf = ECVRF::from_suite(CipherSuite::SECP256K1_SHA256_TAI).unwrap();

    let secret_key = generate_secret_key();

    let public_key = vrf.derive_public_key(&secret_key.secret_bytes()).unwrap();
    let message: &[u8] = b"sample";
    let pi = vrf.prove(&secret_key.secret_bytes(), &message).unwrap();
    pi
}

pub fn verify_seed(seed: &[u8], public_key: &[u8], message: &[u8]) {
    let mut vrf = ECVRF::from_suite(CipherSuite::SECP256K1_SHA256_TAI).unwrap();
    let pi = vrf.prove(&seed, &message).unwrap();
    let hash = vrf.proof_to_hash(&pi).unwrap();
    let beta = vrf.verify(&public_key, &pi, &message);

    let vec2 = beta.unwrap();

    assert_eq!(hash, vec2, "VRF failed");
    
}

fn vec_to_arr(vector: Vec<u8>) -> [u8; 32] {
    let mut arr = [0u8; 32];
    for i in 0..32 {
        arr[i] = vector[i];
    }
    arr
}

pub fn get_rand_num() -> rand_chacha::ChaCha20Rng {
    let input_seed = generate_seed();
    let seed = vec_to_arr(input_seed);
    let rand_num = ChaCha20Rng::from_seed(seed);
    rand_num
}

/*
pub fn get_rand_num_in_range(min: u64, max: u64) -> u64 {
    let mut rng = get_rand_num() ;
    let num = rng.output % (max - min +1) + min;

    num 
}
*/

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        let result = generate_secret_key();
        println!("{:?}", result.display_secret());
        assert!(true);
    }
}
