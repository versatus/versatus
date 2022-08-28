use vrrb_vrf::{vvrf::VVRF, vrng::VRNG};
use rand_chacha::{ChaCha20Rng};
use std::iter::FromIterator;

pub struct DummyNode {
    pub pubkey: String, 
    pub staked: u128,
}

//fewer than 51% w valid pointer sums!
    //integer overflow on u128 (pointer sums are over)
    //get pointer method on claim, if claim hash doesnt match every char in seed, returns none
    //nonce all claims up by 1 and re-run

impl DummyNode {
    pub fn new(message: &[u8]) -> DummyNode{
        let secret_key = VVRF::generate_secret_key();
        let vvrf1 = VVRF::new(message, secret_key);
        
        let mut pk = vvrf1.pubkey;
        let pk_bytes = &pk[0..32];
        let pubkey = std::str::from_utf8(pk_bytes).unwrap().to_string();
        

        let sk = VVRF::generate_secret_key();
        let vvrf2 = VVRF::new(message, sk);
        let min :u128 = 0;
        let max :u128 = 20000;
        let staked = vvrf2.generate_u128_in_range(min: u128, max: u128);

        DummyNode{
            pubkey,
            staked,
        }
    }
}