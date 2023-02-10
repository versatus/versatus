/// Move this module elsewhere
use sha256::digest_bytes;

#[derive(Debug, Clone)]
pub struct Claim {
    pub hash: String,
    pub pointer: Option<u128>,
    pub start: Option<u8>,
}


impl Claim {
    pub fn new(pubkey: String, claim_nonce: u128) -> Claim {
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
            hash,
            pointer: None,
            start: None,
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
}
