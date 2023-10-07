pub mod election;
pub mod quorum;

#[cfg(test)]
mod tests {
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
        net::SocketAddr,
    };

    use primitives::{Address, NodeId};
    use sha256::digest;
    use vrrb_core::{claim::Claim, keypair::KeyPair};

    use crate::{election::Election, quorum::Quorum};

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn not_enough_claims() {
        let mut dummy_claims: Vec<Claim> = Vec::new();

        (0..3).for_each(|_| {
            let keypair = KeyPair::random();
            let public_key = keypair.get_miner_public_key().clone();
            let ip_address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
            let signature = Claim::signature_for_valid_claim(
                public_key.clone(),
                ip_address,
                keypair.get_miner_secret_key().secret_bytes().to_vec(),
            )
            .unwrap();
            let claim: Claim = Claim::new(
                public_key,
                Address::new(public_key),
                ip_address,
                signature,
                NodeId::default(),
            )
            .unwrap();

            //let claim_box = Box::new(claim);
            dummy_claims.push(claim);
        });
        let keypair = KeyPair::random();
        let public_key = keypair.get_miner_public_key();
        let mut hasher = DefaultHasher::new();
        public_key.hash(&mut hasher);
        let pubkey_hash = hasher.finish();

        let mut pub_key_bytes = pubkey_hash.to_string().as_bytes().to_vec();
        pub_key_bytes.push(1u8);

        // Is this double hash neccesary?
        let hash = digest(digest(&*pub_key_bytes).as_bytes());

        let payload1 = (10, hash);

        if let Ok(seed) = Quorum::generate_seed(payload1, keypair.clone()) {
            if let Ok(mut quorum) = Quorum::new(seed, 11, None) {
                assert!(quorum.run_election(dummy_claims).is_err());
            };
        }
    }

    #[test]
    fn invalid_seed_block_height() {
        let mut dummy_claims: Vec<Claim> = Vec::new();

        (0..3).for_each(|_| {
            let keypair = KeyPair::random();
            let public_key = keypair.get_miner_public_key().clone();
            let ip_address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
            let signature = Claim::signature_for_valid_claim(
                public_key.clone(),
                ip_address,
                keypair.get_miner_secret_key().secret_bytes().to_vec(),
            )
            .unwrap();
            let claim: Claim = Claim::new(
                public_key,
                Address::new(public_key),
                ip_address,
                signature,
                NodeId::default(),
            )
            .unwrap();

            dummy_claims.push(claim);
        });
        let keypair = KeyPair::random();
        let public_key = keypair.get_miner_public_key();

        let mut hasher = DefaultHasher::new();
        public_key.hash(&mut hasher);
        let pubkey_hash = hasher.finish();

        let mut pub_key_bytes = pubkey_hash.to_string().as_bytes().to_vec();
        pub_key_bytes.push(1u8);

        let hash = digest(digest(&*pub_key_bytes).as_bytes());

        let payload1 = (0, hash);

        assert!(Quorum::generate_seed(payload1, keypair).is_err());
    }

    #[test]
    fn invalid_seed_block_timestamp() {
        let mut dummy_claims: Vec<Claim> = Vec::new();

        (0..3).for_each(|_| {
            let keypair = KeyPair::random();
            let public_key = keypair.get_miner_public_key().clone();
            let ip_address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
            let signature = Claim::signature_for_valid_claim(
                public_key.clone(),
                ip_address,
                keypair.get_miner_secret_key().secret_bytes().to_vec(),
            )
            .unwrap();
            let claim: Claim = Claim::new(
                public_key,
                Address::new(public_key),
                ip_address,
                signature,
                NodeId::default(),
            )
            .unwrap();
            dummy_claims.push(claim);
        });
        let keypair = KeyPair::random();
        let public_key = keypair.get_miner_public_key();
        let mut hasher = DefaultHasher::new();
        public_key.hash(&mut hasher);
        let pubkey_hash = hasher.finish();

        let mut pub_key_bytes = pubkey_hash.to_string().as_bytes().to_vec();
        pub_key_bytes.push(1u8);

        let hash = digest(digest(&*pub_key_bytes).as_bytes());

        let payload1 = (0, hash);

        assert!(Quorum::generate_seed(payload1, keypair).is_err());
    }

    #[test]
    fn invalid_election_block_height() {
        let mut dummy_claims: Vec<Claim> = Vec::new();
        (0..3).for_each(|_| {
            let keypair = KeyPair::random();
            let public_key = keypair.get_miner_public_key().clone();
            let ip_address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
            let signature = Claim::signature_for_valid_claim(
                public_key.clone(),
                ip_address,
                keypair.get_miner_secret_key().secret_bytes().to_vec(),
            )
            .unwrap();
            let claim: Claim = Claim::new(
                public_key,
                Address::new(public_key),
                ip_address,
                signature,
                NodeId::default(),
            )
            .unwrap();
            dummy_claims.push(claim);
        });
        let keypair = KeyPair::random();
        let public_key = keypair.get_miner_public_key();
        let mut hasher = DefaultHasher::new();
        public_key.hash(&mut hasher);
        let pubkey_hash = hasher.finish();

        let mut pub_key_bytes = pubkey_hash.to_string().as_bytes().to_vec();
        pub_key_bytes.push(1u8);

        let hash = digest(digest(&*pub_key_bytes).as_bytes());

        let payload1 = (10, hash);

        let seed = Quorum::generate_seed(payload1, keypair.clone());

        if let Ok(seed) = seed {
            assert!(Quorum::new(seed, 0, None).is_err());
        }
    }

    #[test]
    fn invalid_election_block_timestamp() {
        let mut dummy_claims: Vec<Claim> = Vec::new();
        (0..20).for_each(|_| {
            let keypair = KeyPair::random();
            let public_key = keypair.get_miner_public_key().clone();
            let ip_address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
            let signature = Claim::signature_for_valid_claim(
                public_key.clone(),
                ip_address,
                keypair.get_miner_secret_key().secret_bytes().to_vec(),
            )
            .unwrap();
            let claim: Claim = Claim::new(
                public_key,
                Address::new(public_key),
                ip_address,
                signature,
                NodeId::default(),
            )
            .unwrap();
            dummy_claims.push(claim);
        });

        let keypair = KeyPair::random();
        let public_key = keypair.get_miner_public_key();
        let mut hasher = DefaultHasher::new();
        public_key.hash(&mut hasher);
        let pubkey_hash = hasher.finish();

        let mut pub_key_bytes = pubkey_hash.to_string().as_bytes().to_vec();
        pub_key_bytes.push(1u8);

        let hash = digest(digest(&*pub_key_bytes).as_bytes());

        let payload1 = (10, hash);

        if let Ok(seed) = Quorum::generate_seed(payload1, keypair.clone()) {
            assert!(Quorum::new(seed, 0, None).is_err());
        }
    }

    #[test]
    #[ignore = "temporarily disabled while the crate is refactored"]
    fn elect_quorum() {
        let mut dummy_claims: Vec<Claim> = Vec::new();
        (0..25).for_each(|_| {
            let keypair = KeyPair::random();
            let public_key = keypair.get_miner_public_key().clone();
            let ip_address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
            let signature = Claim::signature_for_valid_claim(
                public_key.clone(),
                ip_address,
                keypair.get_miner_secret_key().secret_bytes().to_vec(),
            )
            .unwrap();
            let claim: Claim = Claim::new(
                public_key,
                Address::new(public_key),
                ip_address,
                signature,
                NodeId::default(),
            )
            .unwrap();
            dummy_claims.push(claim);
        });
        let keypair = KeyPair::random();
        let public_key = keypair.get_miner_public_key();
        let mut hasher = DefaultHasher::new();
        public_key.hash(&mut hasher);
        let pubkey_hash = hasher.finish();

        let mut pub_key_bytes = pubkey_hash.to_string().as_bytes().to_vec();
        pub_key_bytes.push(1u8);

        let hash = digest(digest(&*pub_key_bytes).as_bytes());

        let payload1 = (10, hash);

        if let Ok(seed) = Quorum::generate_seed(payload1, keypair.clone()) {
            if let Ok(mut quorum) = Quorum::new(seed, 11, None) {
                if quorum.run_election(dummy_claims.clone()).is_ok() {
                    // TODO:
                    // assert!(quorum.master_pubkeys.len() == 13);
                }
            };
        }
    }

    #[test]
    fn elect_identical_quorums() {
        let mut dummy_claims1: Vec<Claim> = Vec::new();
        let mut dummy_claims2: Vec<Claim> = Vec::new();

        (0..3).for_each(|_| {
            let keypair = KeyPair::random();
            let public_key = keypair.get_miner_public_key().clone();
            let ip_address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
            let signature = Claim::signature_for_valid_claim(
                public_key.clone(),
                ip_address,
                keypair.get_miner_secret_key().secret_bytes().to_vec(),
            )
            .unwrap();
            let claim: Claim = Claim::new(
                public_key,
                Address::new(public_key),
                ip_address,
                signature,
                NodeId::default(),
            )
            .unwrap();
            //let boxed_claim = Box::new(claim);
            dummy_claims1.push(claim.clone());
            dummy_claims2.push(claim.clone());
        });

        let keypair = KeyPair::random();
        let public_key = keypair.get_miner_public_key();
        let mut hasher = DefaultHasher::new();
        public_key.hash(&mut hasher);
        let pubkey_hash = hasher.finish();

        let mut pub_key_bytes = pubkey_hash.to_string().as_bytes().to_vec();
        pub_key_bytes.push(1u8);

        let hash = digest(digest(&*pub_key_bytes).as_bytes());

        let payload = (10, hash);

        if let Ok(seed1) = Quorum::generate_seed(payload.clone(), keypair.clone()) {
            if let Ok(seed2) = Quorum::generate_seed(payload.clone(), keypair.clone()) {
                if let Ok(mut quorum1) = Quorum::new(seed1, 11, None) {
                    if let Ok(mut quorum2) = Quorum::new(seed2, 11, None) {
                        if let Ok(q1) = quorum1.run_election(dummy_claims1) {
                            if let Ok(q2) = quorum2.run_election(dummy_claims2) {
                                // TODO
                                // assert!(q1.master_pubkeys == q2.master_pubkeys);
                            }
                        }
                    }
                }
            }
        }
    }
}
