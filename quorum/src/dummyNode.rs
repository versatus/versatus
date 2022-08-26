use vrrb_vrf::{vvrf::VVRF, vrng::VRNG};
use rand_chacha::{ChaCha20Rng};
use std::iter::FromIterator;

pub struct DummyNode {
    pub nodeId: snowflake::ProcessUniqueId,
    pub pubkey: String, 
    pub staked: u128,
}

impl DummyNode {
    pub fn new() -> DummyNode{
        let nodeId = snowflake::ProcessUniqueId::new();

        let secret_key = VVRF::generate_secret_key();
        let vvrf1 = VVRF::new(b"test", secret_key);
        
        let mut pk = vvrf1.pubkey;
        let pk_bytes = &pk[0..32];
        let pubkey = std::str::from_utf8(pk_bytes).unwrap().to_string();
        

        let sk = VVRF::generate_secret_key();
        let vvrf2 = VVRF::new(b"test", sk);
        let min :u128 = 0;
        let max :u128 = 20000;
        let staked = vvrf2.generate_u128_in_range(min: u128, max: u128);

        DummyNode{
            nodeId,
            pubkey,
            staked,
        }
    }

}