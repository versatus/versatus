use vrrb_vrf::{vvrf::VVRF, vrng::VRNG};
use secp256k1::{
    key::{PublicKey, SecretKey},
};
use secp256k1::{Secp256k1};
use sha256::digest_bytes;

#[derive(Clone)]
pub struct DummyNode {
    pub pubkey: String, 
    pub staked: u128,
}

impl DummyNode {
    pub fn new(message: &[u8]) -> DummyNode{

        let secp = Secp256k1::new();
        let mut rng = rand::thread_rng();
   
        let (seckey, pub_key) = secp.generate_keypair(&mut rng);
        let mut pub_key_bytes = pub_key.to_string().as_bytes().to_vec();
        pub_key_bytes.push(1u8);

        let pubkey = digest_bytes(digest_bytes(&pub_key_bytes).as_bytes());


        let sk = VVRF::generate_secret_key();
        let mut vvrf = VVRF::new(message, sk);
        let min :u128 = 0;
        let max :u128 = 20000;
        let staked = vvrf.generate_u128_in_range(min, max);

        return DummyNode{
            pubkey,
            staked,
        }
    }
}