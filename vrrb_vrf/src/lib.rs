//! vrrb verifiable random function library crate
//! 
//! generates a random number 
//!

mod vrrb_vrf {
    #![allow(unused)]
    use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
    use secp256k1::{SecretKey, PublicKey};
    use vrf::openssl::{CipherSuite, ECVRF};
    use vrf::VRF;
    use rand_core::RngCore;

    pub trait VRNG {
        fn generate_u8(&mut self) -> u8;
        fn generate_u16(&mut self) -> u16;
        fn generate_u32(&mut self) -> u32;
        fn generate_u64(&mut self) -> u64;
        fn generate_u128(&mut self) -> u128;
    }

    pub struct VVRF {
        pub vrf: ECVRF,
        pubkey: Vec<u8>,
        message: Vec<u8>,
        proof: [u8; 81],
        hash: [u8; 32],
        rng: ChaCha20Rng,
    }

    impl VRNG for VVRF {
        fn generate_u8(&mut self) -> u8 {
            let mut data = [0u8; 1];
            self.rng.fill_bytes(&mut data);
            u8::from_be_bytes(data)
        }

        fn generate_u16(&mut self) -> u16 {
            let mut data = [0u8; 2];
            self.rng.fill_bytes(&mut data);
            u16::from_be_bytes(data)
        }

        fn generate_u32(&mut self) -> u32 {
            let mut data = [0u8; 4];
            self.rng.fill_bytes(&mut data);
            u32::from_be_bytes(data)
        }

        fn generate_u64(&mut self) -> u64 {
            let mut data = [0u8; 8];
            self.rng.fill_bytes(&mut data);
            u64::from_be_bytes(data)
        }

        fn generate_u128(&mut self) -> u128 {
            let mut data = [0u8; 16];
            self.rng.fill_bytes(&mut data);
            u128::from_be_bytes(data)
        }

    }

    impl VVRF {
        pub fn new(message: &[u8], sk: SecretKey) -> VVRF {
            let mut vrf = VVRF::generate_vrf(CipherSuite::SECP256K1_SHA256_TAI);
            let pubkey = VVRF::generate_pubkey(&mut vrf, sk);
            let (proof, hash) = VVRF::generate_seed(&mut vrf, message, sk).unwrap();
            let rng = ChaCha20Rng::from_seed(hash);
            VVRF { vrf, pubkey, message: message.to_vec(), proof, hash, rng: rng }
        }

        fn generate_secret_key() -> SecretKey {
            let secret_key = SecretKey::new(&mut rand::thread_rng());
            secret_key
        }

        fn generate_vrf(suite: CipherSuite) -> ECVRF {
            ECVRF::from_suite(suite).unwrap()
        }

        fn generate_pubkey(vrf: &mut ECVRF, secret_key: SecretKey) -> Vec<u8> {
            vrf.derive_public_key(&secret_key.secret_bytes()).unwrap()
        }

        fn generate_seed(vrf: &mut ECVRF, message: &[u8], secret_key: SecretKey) -> Option<([u8; 81], [u8; 32])> {
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

        pub fn verify_seed(&mut self) {
            if let Ok(beta) = self.vrf.verify(&self.pubkey, &self.proof, &self.message) {
                assert_eq!(self.hash.to_vec(), beta);
            } else {
                panic!("Error returned by vrf.verify()")
            }
        }

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
    use secp256k1::SecretKey;
    use vrrb_vrf::{VRNG, VVRF};
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
        let beta = vrf.verify(&vvrf.get_pubkey(), &vvrf.get_proof(), &vvrf.get_message()).unwrap();
        let hash = vvrf.get_hash();
        assert_eq!(hash.to_vec(), beta);
    }
}
