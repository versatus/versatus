
use vrrb_vrf::{vvrf::VVRF, vrng::VRNG};

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

        let hash = String::from_utf8(vvrf1.hash.to_vec()).unwrap();
        
        return DummyChildBlock{
            hash, 
            timestamp, 
            height
        }
    }
}
