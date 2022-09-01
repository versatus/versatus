use vrrb_vrf::{vvrf::VVRF, vrng::VRNG};

#[derive(Clone)]
pub struct DummyNode {
    pub pubkey: String, 
    pub staked: u128,
}

impl DummyNode {
    pub fn new(message: &[u8]) -> DummyNode{
        let secret_key = VVRF::generate_secret_key();
        let vvrf1 = VVRF::new(message, secret_key);

        println!("{:?}", vvrf1.pubkey);
        
        let pk = vvrf1.pubkey;

        let pk_bytes = &pk[0..32];
        //bytes are probably not valid utf 8
        let pubkey = String::from_utf8(pk.to_vec()).expect("Found invalid UTF-8");
        println!("{}", pubkey);
        
        let sk = VVRF::generate_secret_key();
        let mut vvrf2 = VVRF::new(message, sk);
        let min :u128 = 0;
        let max :u128 = 20000;
        let staked = vvrf2.generate_u128_in_range(min, max);

        return DummyNode{
            pubkey,
            staked,
        }
    }
}