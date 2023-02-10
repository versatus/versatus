//! vrrb verifiable random function library crate
//!
//! generates a random number or random word/mnemonic phrase

pub mod vrng;
pub mod vvrf;

#[cfg(test)]
mod tests {
    use parity_wordlist::WORDS;
    use vrf::{
        openssl::{CipherSuite, ECVRF},
        VRF,
    };
    use vrrb_core::keypair::KeyPair;

    use crate::{vrng::VRNG, vvrf::VVRF};

    #[test]
    fn same_seed_equals_same_random_8() {
        let kp = KeyPair::random();
        let message = b"test";
        let sk = kp.miner_kp.0.secret_bytes().to_vec();
        let mut vvrf1: VVRF = VVRF::new(message, &sk);
        let mut vvrf2: VVRF = VVRF::new(message, &sk);
        let rn1 = vvrf1.generate_u8();
        let rn2 = vvrf2.generate_u8();
        println!("{:?}", rn1);
        println!("{:?}", rn2);
        assert_eq!(rn1, rn2);
    }

    #[test]
    fn same_seed_equals_same_random_u16() {
        let kp = KeyPair::random();
        let message = b"test";
        let sk = kp.miner_kp.0.secret_bytes().to_vec();
        let mut vvrf1: VVRF = VVRF::new(message, &sk);
        let mut vvrf2: VVRF = VVRF::new(message, &sk);
        let rn1 = vvrf1.generate_u16();
        let rn2 = vvrf2.generate_u16();
        println!("{:?}", rn1);
        println!("{:?}", rn2);
        assert_eq!(rn1, rn2);
    }

    #[test]
    fn same_seed_equals_same_random_32() {
        let kp = KeyPair::random();
        let message = b"test";
        let sk = kp.miner_kp.0.secret_bytes().to_vec();
        let mut vvrf1: VVRF = VVRF::new(message, &sk);
        let mut vvrf2: VVRF = VVRF::new(message, &sk);
        let rn1 = vvrf1.generate_u32();
        let rn2 = vvrf2.generate_u32();
        println!("{:?}", rn1);
        println!("{:?}", rn2);
        assert_eq!(rn1, rn2);
    }
    #[test]
    fn same_seed_equals_same_random_u64() {
        let kp = KeyPair::random();
        let message = b"test";
        let sk = kp.miner_kp.0.secret_bytes().to_vec();
        let mut vvrf1: VVRF = VVRF::new(message, &sk);
        let mut vvrf2: VVRF = VVRF::new(message, &sk);
        let rn1 = vvrf1.generate_u64();
        let rn2 = vvrf2.generate_u64();
        println!("{:?}", rn1);
        println!("{:?}", rn2);
        assert_eq!(rn1, rn2);
    }

    #[test]
    fn same_seed_equals_same_random_u128() {
        let message = b"test";
        let kp = KeyPair::random();
        let sk = kp.miner_kp.0.secret_bytes().to_vec();
        let mut vvrf1: VVRF = VVRF::new(message, &sk);
        let mut vvrf2: VVRF = VVRF::new(message, &sk);
        let rn1 = vvrf1.generate_u128();
        let rn2 = vvrf2.generate_u128();
        println!("{:?}", rn1);
        println!("{:?}", rn2);
        assert_eq!(rn1, rn2);
    }

    #[test]
    fn hash_is_verifiable() {
        let kp = KeyPair::random();
        let message = b"test";
        let sk = kp.miner_kp.0.secret_bytes().to_vec();
        let vvrf: VVRF = VVRF::new(message, &sk);
        let mut vrf = ECVRF::from_suite(CipherSuite::SECP256K1_SHA256_TAI).unwrap();
        let beta = vrf
            .verify(&vvrf.get_pubkey(), &vvrf.get_proof(), &vvrf.get_message())
            .unwrap();
        let hash = vvrf.get_hash();
        assert_eq!(hash.to_vec(), beta);
    }

    #[test]
    fn generates_word_from_lib() {
        let kp = KeyPair::random();
        let message = b"test";
        let sk = kp.miner_kp.0.secret_bytes().to_vec();
        let mut vvrf: VVRF = VVRF::new(message, &sk);
        assert!(WORDS.contains(&(vvrf.generate_word()).as_str()));
    }

    #[test]
    fn generates_right_num_words() {
        let kp = KeyPair::random();
        let message = b"test";
        let sk = kp.miner_kp.0.secret_bytes().to_vec();
        let mut vvrf1: VVRF = VVRF::new(message, &sk);
        assert_eq!((vvrf1.generate_words(7)).len(), 7);
    }

    #[test]
    fn generates_rng_in_range() {
        let kp = KeyPair::random();
        let message = b"test";
        let sk = kp.miner_kp.0.secret_bytes().to_vec();
        let mut vvrf1: VVRF = VVRF::new(message, &sk);
        let rn = vvrf1.generate_u8_in_range(10, 100);
        assert!(10 <= rn && rn <= 100);
    }
}
