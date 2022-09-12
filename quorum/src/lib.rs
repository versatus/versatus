pub  mod election;
pub mod quorum;

#[cfg(test)]
mod tests {
    use claim::claim::Claim;
    use crate::election::Election;
    use crate::quorum::Quorum;
    use secp256k1;
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

        let payload1 = (10, 10, hash);

        if let Ok(seed) = Quorum::generate_seed(payload1){
            if let Ok(mut quorum) = Quorum::new(seed, 11, 11) {
                assert!(quorum.run_election(dummyClaims).is_err());
            };   
        }
    }

    
    #[test]
    fn invalid_seed_block_height() {
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

        let payload1 = (10, 0, hash);

        assert!(Quorum::generate_seed(payload1).is_err());     
    }

    
    #[test]
    fn invalid_seed_block_timestamp() {
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

        let payload1 = (0, 10, hash);

        assert!(Quorum::generate_seed(payload1).is_err());     
    }

    #[test]
    fn invalid_election_block_height() {
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

        let payload1 = (10, 10, hash);

        let seed = Quorum::generate_seed(payload1);

        let mut quorum: Quorum;
        if let Ok(seed) = seed{
            assert!(Quorum::new(seed, 11, 0).is_err());
        }   
    }

    #[test]
    fn invalid_election_block_timestamp() {
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

        let payload1 = (10, 10, hash);

        if let Ok(seed) = Quorum::generate_seed(payload1){
            assert!(Quorum::new(seed, 0, 11).is_err());    
        }
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

        let payload1 = (10, 10, hash);

        if let Ok(seed) = Quorum::generate_seed(payload1){
            if let Ok(mut quorum) = Quorum::new(seed, 11, 11) {
                quorum.run_election(dummyClaims);
                assert!(quorum.master_pubkeys.len() == 13);
            };   
        }
    }
    
    #[test] 
    fn elect_identical_quorums() {
        let mut dummyClaims1: Vec<Claim> = Vec::new();
        let mut dummyClaims2: Vec<Claim> = Vec::new();
        let addr: String = "0x0000000000000000000000000000000000000000".to_string();
        (0..3).for_each(
            |i| {
                let secp = Secp256k1::new();

                let mut rng = rand::thread_rng();
        
                let (secret_key, public_key) = secp.generate_keypair(&mut rng);
                let claim: Claim = Claim::new(public_key.to_string(), addr.clone(), i as u128);
            
                dummyClaims1.push(claim.clone());
                dummyClaims2.push(claim.clone());
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

        if let Ok(seed1) = Quorum::generate_seed(payload.clone()) {
            if let Ok(seed2) = Quorum::generate_seed(payload.clone()){
                if let Ok(mut quorum1) = Quorum::new(seed1, 11, 11) {
                    if let Ok(mut quorum2) = Quorum::new(seed2, 11, 11) {
                        quorum1.run_election(dummyClaims1);
                        quorum2.run_election(dummyClaims2);
                        assert!(quorum1.master_pubkeys == quorum2.master_pubkeys);
                    }
                }
            }
        }
    } 
}
