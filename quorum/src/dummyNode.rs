use vrrb_vrf::vvrf::VVRF;

pub struct DummyNode {
    pub nodeId: u64,
    pub pubkey: String, 
    pub staked: u64,
}

impl DumyyNode {
    pub fn new(){
        let nodeId = ProcessUniqueId::new();

        let secret_key = SecretKey::new(&mut rand::thread_rng());
        let mut vrf = VVRF::generate_vrf(CipherSuite::SECP256K1_SHA256_TAI);
        let mut pubkey = VVRF::generate_pubkey(&mut vrf, secret_key);
        pubkey.unwrap().trim_start().to_string();

        


    }

}