
use vrrb_vrf::{vvrf::VVRF, vrng::VRNG};
use secp256k1::{
    key::{PublicKey, SecretKey},
};
use secp256k1::{Secp256k1};
use sha256::digest_bytes;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct DummyChildBlock{
    pub hash: String, 
    pub timestamp: u128, 
    pub height: u128
}

impl DummyChildBlock {
    pub fn new(message1: &[u8], message2: &[u8]) -> DummyChildBlock{
        let secret_key1 = VVRF::generate_secret_key();
        let mut vvrf1 = VVRF::new(message1, secret_key1);

        let secret_key2 = VVRF::generate_secret_key();
        let mut vvrf2 = VVRF::new(message2, secret_key2);
        
        let timestamp = vvrf1.generate_u128();
        let height = vvrf2.generate_u128();

        let secp = Secp256k1::new();

        let mut rng = rand::thread_rng();

        let (secret_key, public_key) = secp.generate_keypair(&mut rng);

        let mut hasher = DefaultHasher::new();
        public_key.hash(&mut hasher);
        let pubkey_hash = hasher.finish();

        let mut pub_key_bytes = pubkey_hash.to_string().as_bytes().to_vec();
        pub_key_bytes.push(1u8);

        let hash = digest_bytes(digest_bytes(&pub_key_bytes).as_bytes());
        
        return DummyChildBlock{
            hash, 
            timestamp, 
            height
        }
    }
}
