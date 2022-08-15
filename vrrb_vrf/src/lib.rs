//! vrrb verifiable random function library crate
//!
//! generates a random number or random word/mnemonic phrase

pub mod vrng;
pub mod vvrf;
//use crate::vrrb_vrf;

///root module


#[cfg(test)]
mod tests {
    use super::*;
    use crate::vrng::VRNG;
    use crate::vvrf::VVRF;
    use parity_wordlist::WORDS;
    use secp256k1::SecretKey;
    use vrf::openssl::{CipherSuite, ECVRF};
    use vrf::VRF;

    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }

    #[test]
    fn same_seed_equals_same_random_8() {
        let sk = SecretKey::new(&mut rand::thread_rng());
        let message = b"test";
        let mut vvrf1: VVRF = VVRF::new(message, sk);
        let mut vvrf2: VVRF = VVRF::new(message, sk);
        let rn1 = vvrf1.generate_u8();
        let rn2 = vvrf2.generate_u8();
        println!("{:?}", rn1);
        println!("{:?}", rn2);
        assert_eq!(rn1, rn2);
    }

    #[test]
    fn same_seed_equals_same_random_u16() {
        let sk = SecretKey::new(&mut rand::thread_rng());
        let message = b"test";
        let mut vvrf1: VVRF = VVRF::new(message, sk);
        let mut vvrf2: VVRF = VVRF::new(message, sk);
        let rn1 = vvrf1.generate_u16();
        let rn2 = vvrf2.generate_u16();
        println!("{:?}", rn1);
        println!("{:?}", rn2);
        assert_eq!(rn1, rn2);
    }

    #[test]
    fn same_seed_equals_same_random_32() {
        let sk = SecretKey::new(&mut rand::thread_rng());
        let message = b"test";
        let mut vvrf1: VVRF = VVRF::new(message, sk);
        let mut vvrf2: VVRF = VVRF::new(message, sk);
        let rn1 = vvrf1.generate_u32();
        let rn2 = vvrf2.generate_u32();
        println!("{:?}", rn1);
        println!("{:?}", rn2);
        assert_eq!(rn1, rn2);
    }
    #[test]
    fn same_seed_equals_same_random_u64() {
        let sk = SecretKey::new(&mut rand::thread_rng());
        let message = b"test";
        let mut vvrf1: VVRF = VVRF::new(message, sk);
        let mut vvrf2: VVRF = VVRF::new(message, sk);
        let rn1 = vvrf1.generate_u64();
        let rn2 = vvrf2.generate_u64();
        println!("{:?}", rn1);
        println!("{:?}", rn2);
        assert_eq!(rn1, rn2);
    }

    #[test]
    fn same_seed_equals_same_random_u128() {
        let sk = SecretKey::new(&mut rand::thread_rng());
        let message = b"test";
        let mut vvrf1: VVRF = VVRF::new(message, sk);
        let mut vvrf2: VVRF = VVRF::new(message, sk);
        let rn1 = vvrf1.generate_u128();
        let rn2 = vvrf2.generate_u128();
        println!("{:?}", rn1);
        println!("{:?}", rn2);
        assert_eq!(rn1, rn2);
    }

    #[test]
    fn hash_is_verifiable() {
        let sk = SecretKey::new(&mut rand::thread_rng());
        let message = b"test";
        let vvrf: VVRF = VVRF::new(message, sk);
        let mut vrf = ECVRF::from_suite(CipherSuite::SECP256K1_SHA256_TAI).unwrap();
        let beta = vrf
            .verify(&vvrf.get_pubkey(), &vvrf.get_proof(), &vvrf.get_message())
            .unwrap();
        let hash = vvrf.get_hash();
        assert_eq!(hash.to_vec(), beta);
    }

    #[test]
    fn generates_word_from_lib() {
        let sk = SecretKey::new(&mut rand::thread_rng());
        let message = b"test";
        let mut vvrf: VVRF = VVRF::new(message, sk);
        let word = (vvrf.generate_word()).as_str();
        assert!(WORDS.contains(&word));
    }

    #[test]
    fn generates_right_num_words() {
        let sk = SecretKey::new(&mut rand::thread_rng());
        let message = b"test";
        let mut vvrf1: VVRF = VVRF::new(message, sk);
        assert_eq!((vvrf1.generate_words(7)).len(), 7);
    }
}
