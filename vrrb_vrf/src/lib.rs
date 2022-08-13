//! vrrb verifiable random function library crate
//!
//! generates a random number or random word/mnemonic phrase

pub mod vrng;
pub mod vvrf;

///root module
mod vrrb_vrf {
    #![allow(unused)]
    use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
    use rand_core::RngCore;
    use secp256k1::{PublicKey, SecretKey};
    use vrf::openssl::{CipherSuite, ECVRF};
    use vrf::VRF;

    use crate::vrng::VRNG;
    use crate::vvrf::{InvalidProofError, VVRF};

    ///implement VVRF type by passing a secretKey such that
    ///all the VVRF fields can now be calculated thanks to the sk being passed
    ///and use of fxns defined below and imported
    impl VVRF {
        pub fn new(message: &[u8], sk: SecretKey) -> VVRF {
            let mut vrf = VVRF::generate_vrf(CipherSuite::SECP256K1_SHA256_TAI);
            let pubkey = VVRF::generate_pubkey(&mut vrf, sk);
            let (proof, hash) = VVRF::generate_seed(&mut vrf, message, sk).unwrap();
            ///rng calculated from hash
            let rng = ChaCha20Rng::from_seed(hash);
            ///populate VVRF fields
            VVRF {
                vrf,
                pubkey,
                message: message.to_vec(),
                proof,
                hash,
                rng: rng,
            }
        }

        ///get a sk using SecretKey struct
        fn generate_secret_key() -> SecretKey {
            let secret_key = SecretKey::new(&mut rand::thread_rng());
            secret_key
        }

        ///get vrf from openssl struct ECVRF (eliptic curve vrf)
        fn generate_vrf(suite: CipherSuite) -> ECVRF {
            ECVRF::from_suite(suite).unwrap()
        }

        ///get pk from vrf crate
        fn generate_pubkey(vrf: &mut ECVRF, secret_key: SecretKey) -> Vec<u8> {
            vrf.derive_public_key(&secret_key.secret_bytes()).unwrap()
        }

        ///generate seed
        fn generate_seed(
            vrf: &mut ECVRF,
            message: &[u8],
            secret_key: SecretKey,
        ) -> Option<([u8; 81], [u8; 32])> {
            if let Ok(pi) = vrf.prove(&secret_key.secret_bytes(), message) {
                if let Ok(hash) = vrf.proof_to_hash(&pi) {
                    let mut proof_buff = [0u8; 81];
                    pi.iter().enumerate().for_each(|(i, v)| {
                        proof_buff[i] = *v;
                    });
                    let mut hash_buff = [0u8; 32];
                    hash.iter().enumerate().for_each(|(i, v)| {
                        hash_buff[i] = *v;
                    });

                    Some((proof_buff, hash_buff))
                } else {
                    None
                }
            } else {
                None
            }
        }

        ///check that hash and beta are equal to ensure hash(seed) is valid
        pub fn verify_seed(&mut self) -> Result<(), InvalidProofError> {
            if let Ok(beta) = self.vrf.verify(&self.pubkey, &self.proof, &self.message) {
                if self.hash.to_vec() != beta {
                    return Err(InvalidProofError);
                } else {
                    return Ok(());
                }
            } else {
                return Err(InvalidProofError);
            }
        }

        ///getter functions
        pub fn get_pubkey(&self) -> Vec<u8> {
            self.pubkey.clone()
        }

        pub fn get_message(&self) -> Vec<u8> {
            self.message.clone()
        }

        pub fn get_proof(&self) -> [u8; 81] {
            self.proof
        }

        pub fn get_hash(&self) -> [u8; 32] {
            self.hash
        }

        pub fn get_rng(&self) -> ChaCha20Rng {
            self.rng.clone()
        }
    }
}

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
