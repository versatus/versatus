pub  mod election;
pub mod quorum;

#[cfg(test)]
mod tests {
    use claim::claim::Claim;
    use crate::election::Election;
    use crate::quorum::Quorum;
    use format_bytes::format_bytes;
    use vrrb_vrf::{vvrf::VVRF, vrng::VRNG};
    use secp256k1::{
        key::{PublicKey, SecretKey},
    };
    use secp256k1::{Secp256k1};
    use sha256::digest_bytes;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};  

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn not_enough_claims() {
        let mut dummyClaims: Vec<Claim> = Vec::new();
        let addr: String = "0x0000000000000000000000000000000000000000".to_string();
        (0..3).for_each(
            |i| {
                let secp = Secp256k1::new();

                let mut rng = rand::thread_rng();
        
                let (secret_key, public_key) = secp.generate_keypair(&mut rng);
                let claim: Claim = Claim::new(public_key.to_string(), addr.clone(), i as u128);
            
                dummyClaims.push(claim);
            }
        );
        let secp = Secp256k1::new();

        let mut rng = rand::thread_rng();

        let (secret_key, public_key) = secp.generate_keypair(&mut rng);

        let mut hasher = DefaultHasher::new();
        public_key.hash(&mut hasher);
        let pubkey_hash = hasher.finish();

        let mut pub_key_bytes = pubkey_hash.to_string().as_bytes().to_vec();
        pub_key_bytes.push(1u8);

        let hash = digest_bytes(digest_bytes(&pub_key_bytes).as_bytes());

        let payload = (10, 10, hash);
        
        let mut quorum: Quorum = Quorum::new();

        assert!(quorum.run_election(payload, dummyClaims).is_err());

    }

    #[test]
    fn invalid_block_height() {
        let mut dummyClaims: Vec<Claim> = Vec::new();
        let addr: String = "0x0000000000000000000000000000000000000000".to_string();
        (0..20).for_each(
            |i| {
                let secp = Secp256k1::new();

                let mut rng = rand::thread_rng();
        
                let (secret_key, public_key) = secp.generate_keypair(&mut rng);
                let claim: Claim = Claim::new(public_key.to_string(), addr.clone(), i as u128);
            
                dummyClaims.push(claim);
            }
        );
        let secp = Secp256k1::new();

        let mut rng = rand::thread_rng();

        let (secret_key, public_key) = secp.generate_keypair(&mut rng);

        let mut hasher = DefaultHasher::new();
        public_key.hash(&mut hasher);
        let pubkey_hash = hasher.finish();

        let mut pub_key_bytes = pubkey_hash.to_string().as_bytes().to_vec();
        pub_key_bytes.push(1u8);

        let hash = digest_bytes(digest_bytes(&pub_key_bytes).as_bytes());

        let payload = (10, 0, hash);
        
        let mut quorum: Quorum = Quorum::new();

        assert!(quorum.run_election(payload, dummyClaims).is_err());
        
    }

    #[test]
    fn invalid_block_timestamp() {
        let mut dummyClaims: Vec<Claim> = Vec::new();
        let addr: String = "0x0000000000000000000000000000000000000000".to_string();
        (0..25).for_each(
            |i| {
                let secp = Secp256k1::new();

                let mut rng = rand::thread_rng();
        
                let (secret_key, public_key) = secp.generate_keypair(&mut rng);
                let claim: Claim = Claim::new(public_key.to_string(), addr.clone(), i as u128);
            
                dummyClaims.push(claim);
            }
        );
        let secp = Secp256k1::new();

        let mut rng = rand::thread_rng();

        let (secret_key, public_key) = secp.generate_keypair(&mut rng);

        let mut hasher = DefaultHasher::new();
        public_key.hash(&mut hasher);
        let pubkey_hash = hasher.finish();

        let mut pub_key_bytes = pubkey_hash.to_string().as_bytes().to_vec();
        pub_key_bytes.push(1u8);

        let hash = digest_bytes(digest_bytes(&pub_key_bytes).as_bytes());

        let payload = (0, 10, hash);
        
        let mut quorum: Quorum = Quorum::new();

        assert!(quorum.run_election(payload, dummyClaims).is_err());
        
    }

    #[test]
    fn elect_quorum() {
        let mut dummyClaims: Vec<Claim> = Vec::new();
        let addr: String = "0x0000000000000000000000000000000000000000".to_string();
        (0..25).for_each(
            |i| {
                let secp = Secp256k1::new();

                let mut rng = rand::thread_rng();
        
                let (secret_key, public_key) = secp.generate_keypair(&mut rng);
                let claim: Claim = Claim::new(public_key.to_string(), addr.clone(), i as u128);
            
                dummyClaims.push(claim);
            }
        );
        let secp = Secp256k1::new();

        let mut rng = rand::thread_rng();

        let (secret_key, public_key) = secp.generate_keypair(&mut rng);

        let mut hasher = DefaultHasher::new();
        public_key.hash(&mut hasher);
        let pubkey_hash = hasher.finish();

        let mut pub_key_bytes = pubkey_hash.to_string().as_bytes().to_vec();
        pub_key_bytes.push(1u8);

        let hash = digest_bytes(digest_bytes(&pub_key_bytes).as_bytes());

        let payload = (10, 10, hash);
        
        let mut quorum: Quorum = Quorum::new();
        quorum.run_election(payload, dummyClaims);

        assert!(quorum.master_pubkeys.len() == 13);
        
    }  
}

