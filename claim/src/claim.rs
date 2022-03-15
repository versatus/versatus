use noncing::nonceable::Nonceable;
use ownable::ownable::Ownable;
use serde::{Deserialize, Serialize};
use serde_json;
use sha256::digest_bytes;
use verifiable::verifiable::Verifiable;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidClaimError {
    details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct Claim {
    pub pubkey: String,
    pub address: String,
    pub hash: String,
    pub nonce: u128,
    pub eligible: bool,
}

impl Claim {
    pub fn new(pubkey: String, address: String, claim_nonce: u128) -> Claim {
        let iters = if let Some(n) = claim_nonce.checked_mul(10) {
            n
        } else {
            claim_nonce
        };

        let mut hash = pubkey.clone();
        (0..iters).for_each(|_| {
            hash = digest_bytes(hash.as_bytes());
        });

        Claim {
            pubkey,
            address,
            hash: hash,
            nonce: claim_nonce,
            eligible: true,
        }
    }

    pub fn get_pointer(&self, nonce: u128) -> Option<u128> {
        let nonce_hex = format!("{:x}", nonce);
        let nonce_string_len = nonce_hex.chars().count();
        let mut pointers = vec![];
        nonce_hex.chars().enumerate().for_each(|(idx, c)| {
            let res = self.hash.find(c);
            if let Some(n) = res {
                let n = n as u128;
                let n = n.checked_pow(idx as u32);
                if let Some(n) = n {
                    pointers.push(n as u128);
                }
            }
        });

        if pointers.len() == nonce_string_len {
            let pointer: u128 = pointers.iter().sum();
            Some(pointer)
        } else {
            None
        }
    }

    pub fn from_string(claim_string: String) -> Claim {
        serde_json::from_str::<Claim>(&claim_string).unwrap()
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        serde_json::to_string(self).unwrap().as_bytes().to_vec()
    }

    pub fn from_bytes(data: &[u8]) -> Claim {
        serde_json::from_slice::<Claim>(data).unwrap()
    }

    pub fn get_field_names(&self) -> Vec<String> {
        vec![
            "pubkey".to_string(),
            "address".to_string(),
            "hash".to_string(),
            "nonce".to_string(),
            "eligible".to_string(),
        ]
    }
}

impl Verifiable for Claim {
    type Item = Option<Vec<u8>>;
    type DependantOne = Option<Vec<u8>>;
    type DependantTwo = Option<Vec<u8>>;
    type Error = InvalidClaimError;
    fn verifiable(&self) -> bool {
        true
    }

    #[allow(unused_variables)]
    fn valid(
        &self,
        item: &Self::Item,
        dependant_one: &Self::DependantOne,
        dependant_two: &Self::DependantTwo,
    ) -> Result<bool, InvalidClaimError> {
        Ok(true)
    }
}

impl Ownable for Claim {
    fn get_pubkey(&self) -> String {
        self.pubkey.clone()
    }
}

impl Nonceable for Claim {
    fn nonceable(&self) -> bool {
        true
    }

    fn nonce_up(&mut self) {
        self.nonce = self.nonce + 1;
        let iters = if let Some(n) = self.nonce.clone().checked_mul(10) {
            n
        } else {
            self.nonce.clone()
        };

        let mut hash = self.pubkey.clone();
        (0..iters).for_each(|_| {
            hash = digest_bytes(hash.as_bytes());
        });

        self.hash = hash;
    }
}
