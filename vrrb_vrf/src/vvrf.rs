use crate::vrng::VRNG;
use vrf::openssl::{CipherSuite, ECVRF};
use rand_chacha::{ChaCha20Rng};
use rand_core::RngCore;
use parity_wordlist::WORDS;
use std::fmt::{Display};
use std::error::Error;
use rand::seq::SliceRandom;

#[derive(Debug)]
pub struct InvalidProofError;

impl Display for InvalidProofError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Invalid Proof for vrf")
    }
}

impl Error for InvalidProofError{
    fn description(&self) -> &str {
        "Invalid Proof for vrf"
    }
}

///VVRF type contains all params necessary for creating and verifying an rng 
///It does not include the secret key 
pub struct VVRF {
    pub vrf: ECVRF,
    pub pubkey: Vec<u8>,
    pub message: Vec<u8>,
    pub proof: [u8; 81],
    pub hash: [u8; 32],
    pub rng: ChaCha20Rng,
}

///implenent VRNG trait for VVRF s.t. VVRF can accomomdate 
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

    fn generate_usize(&mut self) -> usize {
        let mut data = &[0u8; 8];
        let (int_bytes, _) = data.split_at(std::mem::size_of::<usize>());
        usize::from_be_bytes(int_bytes.try_into().unwrap())
    }

    fn generate_word(&mut self) -> String {
        let mut rng = self.rng.clone();
        WORDS.choose(&mut rng).unwrap().trim_start().to_string()
    }

    fn generate_words(&mut self, n: usize) -> Vec<String> {
        let mut rng = self.rng.clone();
        (0..n).map(|_| WORDS.choose(&mut rng).unwrap().to_string()).collect::<Vec<_>>()
    }

    fn generate_phrase(&mut self, n: usize) -> String {
        let mut rng = self.rng.clone();
        (0..n).map(|_| WORDS.choose(&mut rng).unwrap()).fold(String::new(), |mut acc, word| {
            acc.push_str(" ");
            acc.push_str(word);
            acc
        }).trim_start().to_string()
    }
}
