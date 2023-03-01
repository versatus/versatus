pub mod credit_model;
pub mod election;
pub mod quorum;

#[cfg(test)]
mod tests {
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
    };

    use sha256::digest;
    use vrrb_core::{claim::Claim, keypair::KeyPair};

    use crate::{election::Election, quorum::Quorum};

    static TEST_ADDR: &str = "0x0000000000000000000000000000000000000000";

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn not_enough_claims() {
        let mut dummy_claims: Vec<Claim> = Vec::new();

        (0..3).for_each(|i| {
            let keypair = KeyPair::random();
            let public_key = keypair.get_miner_public_key().serialize().to_vec();
            let claim: Claim =
                Claim::new(hex::encode(public_key), TEST_ADDR.to_string(), i as u128);

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

        let payload1 = (10, 10, hash);

        if let Ok(seed) = Quorum::generate_seed(payload1, keypair.clone()) {
            if let Ok(mut quorum) = Quorum::new(seed, 11, 11, keypair) {
                assert!(quorum.run_election(dummy_claims).is_err());
            };
        }
    }

    #[test]
    fn invalid_seed_block_height() {
        let mut dummy_claims: Vec<Claim> = Vec::new();

        (0..3).for_each(|i| {
            let keypair = KeyPair::random();
            let public_key = keypair.get_miner_public_key();
            let claim: Claim = Claim::new(public_key.to_string(), TEST_ADDR.to_string(), i as u128);

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

        let payload1 = (10, 0, hash);

        assert!(Quorum::generate_seed(payload1, keypair).is_err());
    }

    #[test]
    fn invalid_seed_block_timestamp() {
        let mut dummy_claims: Vec<Claim> = Vec::new();

        (0..3).for_each(|i| {
            let keypair = KeyPair::random();
            let public_key = keypair.get_miner_public_key();
            let claim: Claim = Claim::new(public_key.to_string(), TEST_ADDR.to_string(), i as u128);

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

        let payload1 = (0, 10, hash);

        assert!(Quorum::generate_seed(payload1, keypair).is_err());
    }

    #[test]
    fn invalid_election_block_height() {
        let mut dummy_claims: Vec<Claim> = Vec::new();
        (0..3).for_each(|i| {
            let keypair = KeyPair::random();
            let public_key = keypair.get_miner_public_key();
            let claim: Claim = Claim::new(public_key.to_string(), TEST_ADDR.to_string(), i as u128);

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

        let payload1 = (10, 10, hash);

        let seed = Quorum::generate_seed(payload1, keypair.clone());

        if let Ok(seed) = seed {
            assert!(Quorum::new(seed, 11, 0, keypair).is_err());
        }
    }

    #[test]
    fn invalid_election_block_timestamp() {
        let mut dummy_claims: Vec<Claim> = Vec::new();
        (0..20).for_each(|i| {
            let keypair = KeyPair::random();
            let public_key = keypair.get_miner_public_key();
            let claim: Claim = Claim::new(public_key.to_string(), TEST_ADDR.to_string(), i as u128);
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

        let payload1 = (10, 10, hash);

        if let Ok(seed) = Quorum::generate_seed(payload1, keypair.clone()) {
            assert!(Quorum::new(seed, 0, 11, keypair).is_err());
        }
    }

    #[test]
    #[ignore = "temporarily disabled while the crate is refactored"]
    fn elect_quorum() {
        let mut dummy_claims: Vec<Claim> = Vec::new();
        (0..25).for_each(|i| {
            let keypair = KeyPair::random();
            let public_key = keypair.get_miner_public_key();
            let claim: Claim = Claim::new(public_key.to_string(), TEST_ADDR.to_string(), i as u128);
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

        let payload1 = (10, 10, hash);

        if let Ok(seed) = Quorum::generate_seed(payload1, keypair.clone()) {
            if let Ok(mut quorum) = Quorum::new(seed, 11, 11, keypair.clone()) {
                if quorum.run_election(dummy_claims.clone()).is_ok() {
                    assert!(quorum.master_pubkeys.len() == 13);
                } else {
                    //first run w dummy claims, THEN if that fails enter loop
                    let new_claims1 = quorum
                        .nonce_claims_and_new_seed(dummy_claims, keypair.clone())
                        .unwrap();
                    if quorum.run_election(new_claims1.clone()).is_err() {
                        let new_claims2 = quorum
                            .nonce_claims_and_new_seed(new_claims1.clone(), keypair.clone())
                            .unwrap();
                        while quorum.run_election(new_claims2.clone()).is_err() {
                            let new_claims2 = quorum
                                .nonce_claims_and_new_seed(new_claims2.clone(), keypair.clone())
                                .unwrap();
                        }
                    }
                    assert!(quorum.master_pubkeys.len() == 13);
                }
            };
        }
    }

    #[test]
    fn elect_identical_quorums() {
        let mut dummy_claims1: Vec<Claim> = Vec::new();
        let mut dummy_claims2: Vec<Claim> = Vec::new();

        (0..3).for_each(|i| {
            let keypair = KeyPair::random();
            let public_key = keypair.get_miner_public_key();
            let claim: Claim = Claim::new(
                public_key.to_string(),
                TEST_ADDR.to_string().clone(),
                i as u128,
            );
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

        let payload = (10, 10, hash);

        if let Ok(seed1) = Quorum::generate_seed(payload.clone(), keypair.clone()) {
            if let Ok(seed2) = Quorum::generate_seed(payload.clone(), keypair.clone()) {
                if let Ok(mut quorum1) = Quorum::new(seed1, 11, 11, keypair.clone()) {
                    if let Ok(mut quorum2) = Quorum::new(seed2, 11, 11, keypair) {
                        if let Ok(q1) = quorum1.run_election(dummy_claims1) {
                            if let Ok(q2) = quorum2.run_election(dummy_claims2) {
                                assert!(q1.master_pubkeys == q2.master_pubkeys);
                            }
                        }
                    }
                }
            }
        }
    }
}
