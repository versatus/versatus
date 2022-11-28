pub mod election;
pub mod quorum;

#[cfg(test)]
mod tests {
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
    };

    use claim::claim::Claim;
    use secp256k1::{self, Secp256k1};
    use sha256::digest;

    use crate::{election::Election, quorum::{Quorum, InvalidQuorum}};

    static TEST_ADDR: &'static str = &("0x0000000000000000000000000000000000000000");
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn not_enough_claims() {
        let mut dummy_claims: Vec<Claim> = Vec::new();

        (0..3).for_each(|i| {
            let secp = Secp256k1::new();

            let mut rng = rand::thread_rng();

            let (_secret_key, public_key) = secp.generate_keypair(&mut rng);
            let claim: Claim = Claim::new(
                public_key.to_string(),
                TEST_ADDR.to_string().clone(),
                i as u128,
            );

            //let claim_box = Box::new(claim);
            dummy_claims.push(claim);
        });
        let secp = Secp256k1::new();

        let mut rng = rand::thread_rng();

        let (_secret_key, public_key) = secp.generate_keypair(&mut rng);

        let mut hasher = DefaultHasher::new();
        public_key.hash(&mut hasher);
        let pubkey_hash = hasher.finish();

        let mut pub_key_bytes = pubkey_hash.to_string().as_bytes().to_vec();
        pub_key_bytes.push(1u8);

        // Is this double hash neccesary?
        let hash = digest(digest(&*pub_key_bytes).as_bytes());

        let payload1 = (10, 10, hash);

        if let Ok(seed) = Quorum::generate_seed(payload1) {
            if let Ok(mut quorum) = Quorum::new(seed, 11, 11) {
                assert!(quorum.run_election(dummy_claims).is_err());
            };
        }
    }

    #[test]
    fn invalid_seed_block_height() {
        let mut dummy_claims: Vec<Claim> = Vec::new();

        (0..3).for_each(|i| {
            let secp = Secp256k1::new();

            let mut rng = rand::thread_rng();

            let (_secret_key, public_key) = secp.generate_keypair(&mut rng);
            let claim: Claim = Claim::new(
                public_key.to_string(),
                TEST_ADDR.to_string().clone(),
                i as u128,
            );

            dummy_claims.push(claim);
        });
        let secp = Secp256k1::new();

        let mut rng = rand::thread_rng();

        let (_secret_key, public_key) = secp.generate_keypair(&mut rng);

        let mut hasher = DefaultHasher::new();
        public_key.hash(&mut hasher);
        let pubkey_hash = hasher.finish();

        let mut pub_key_bytes = pubkey_hash.to_string().as_bytes().to_vec();
        pub_key_bytes.push(1u8);

        let hash = digest(digest(&*pub_key_bytes).as_bytes());

        let payload1 = (10, 0, hash);

        assert!(Quorum::generate_seed(payload1).is_err());
    }

    #[test]
    fn invalid_seed_block_timestamp() {
        let mut dummy_claims: Vec<Claim> = Vec::new();

        (0..3).for_each(|i| {
            let secp = Secp256k1::new();

            let mut rng = rand::thread_rng();

            let (_secret_key, public_key) = secp.generate_keypair(&mut rng);
            let claim: Claim = Claim::new(
                public_key.to_string(),
                TEST_ADDR.to_string().clone(),
                i as u128,
            );

            dummy_claims.push(claim);
        });
        let secp = Secp256k1::new();

        let mut rng = rand::thread_rng();

        let (_secret_key, public_key) = secp.generate_keypair(&mut rng);

        let mut hasher = DefaultHasher::new();
        public_key.hash(&mut hasher);
        let pubkey_hash = hasher.finish();

        let mut pub_key_bytes = pubkey_hash.to_string().as_bytes().to_vec();
        pub_key_bytes.push(1u8);

        let hash = digest(digest(&*pub_key_bytes).as_bytes());

        let payload1 = (0, 10, hash);

        assert!(Quorum::generate_seed(payload1).is_err());
    }

    #[test]
    fn invalid_election_block_height() {
        let mut dummy_claims: Vec<Claim> = Vec::new();
        (0..3).for_each(|i| {
            let secp = Secp256k1::new();

            let mut rng = rand::thread_rng();

            let (_secret_key, public_key) = secp.generate_keypair(&mut rng);
            let claim: Claim = Claim::new(
                public_key.to_string(),
                TEST_ADDR.to_string().clone(),
                i as u128,
            );

            dummy_claims.push(claim);
        });
        let secp = Secp256k1::new();

        let mut rng = rand::thread_rng();

        let (_secret_key, public_key) = secp.generate_keypair(&mut rng);

        let mut hasher = DefaultHasher::new();
        public_key.hash(&mut hasher);
        let pubkey_hash = hasher.finish();

        let mut pub_key_bytes = pubkey_hash.to_string().as_bytes().to_vec();
        pub_key_bytes.push(1u8);

        let hash = digest(digest(&*pub_key_bytes).as_bytes());

        let payload1 = (10, 10, hash);

        let seed = Quorum::generate_seed(payload1);

        if let Ok(seed) = seed {
            assert!(Quorum::new(seed, 11, 0).is_err());
        }
    }

    #[test]
    fn invalid_election_block_timestamp() {
        let mut dummy_claims: Vec<Claim> = Vec::new();
        (0..20).for_each(|i| {
            let secp = Secp256k1::new();

            let mut rng = rand::thread_rng();

            let (_secret_key, public_key) = secp.generate_keypair(&mut rng);
            let claim: Claim = Claim::new(
                public_key.to_string(),
                TEST_ADDR.to_string().clone(),
                i as u128,
            );

            dummy_claims.push(claim);
        });
        let secp = Secp256k1::new();

        let mut rng = rand::thread_rng();

        let (_secret_key, public_key) = secp.generate_keypair(&mut rng);

        let mut hasher = DefaultHasher::new();
        public_key.hash(&mut hasher);
        let pubkey_hash = hasher.finish();

        let mut pub_key_bytes = pubkey_hash.to_string().as_bytes().to_vec();
        pub_key_bytes.push(1u8);

        let hash = digest(digest(&*pub_key_bytes).as_bytes());

        let payload1 = (10, 10, hash);

        if let Ok(seed) = Quorum::generate_seed(payload1) {
            assert!(Quorum::new(seed, 0, 11).is_err());
        }
    }

    #[test]
    fn elect_quorum() {
        let mut dummy_claims: Vec<Claim> = Vec::new();
        (0..25).for_each(|i| {
            let secp = Secp256k1::new();

            let mut rng = rand::thread_rng();

            let (_secret_key, public_key) = secp.generate_keypair(&mut rng);
            let claim: Claim = Claim::new(
                public_key.to_string(),
                TEST_ADDR.to_string().clone(),
                i as u128,
            );

            //let boxed_claim = Box::new(claim);

            dummy_claims.push(claim);
        });
        let secp = Secp256k1::new();

        let mut rng = rand::thread_rng();

        let (_secret_key, public_key) = secp.generate_keypair(&mut rng);

        let mut hasher = DefaultHasher::new();
        public_key.hash(&mut hasher);
        let pubkey_hash = hasher.finish();

        let mut pub_key_bytes = pubkey_hash.to_string().as_bytes().to_vec();
        pub_key_bytes.push(1u8);

        let hash = digest(digest(&*pub_key_bytes).as_bytes());

        let payload1 = (10, 10, hash);

        if let Ok(seed) = Quorum::generate_seed(payload1) {
            if let Ok(mut quorum) = Quorum::new(seed, 11, 11) {
                if quorum.run_election(dummy_claims.clone()).is_ok() {
                    assert!(quorum.master_pubkeys.len() == 13);
                } else {
                    //first run w dummy claims, THEN if that fails enter loop
                    let new_claims1 = quorum.nonce_claims_and_new_seed(dummy_claims).unwrap();
                    if quorum.run_election(new_claims1.clone()).is_err(){
                        let new_claims2 = quorum.nonce_claims_and_new_seed(new_claims1.clone()).unwrap();
                        //let nonced_up_claims: Vec<Claim> = Vec::new();
                        while quorum.run_election(new_claims2.clone()).is_err() {
                            let new_claims2 = quorum.nonce_claims_and_new_seed(new_claims2.clone()).unwrap();
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
            let secp = Secp256k1::new();

            let mut rng = rand::thread_rng();

            let (_secret_key, public_key) = secp.generate_keypair(&mut rng);
            let claim: Claim = Claim::new(
                public_key.to_string(),
                TEST_ADDR.to_string().clone(),
                i as u128,
            );
            dbg!("CLAIM NONCE:", claim.nonce);
            //let boxed_claim = Box::new(claim);
            dummy_claims1.push(claim.clone());
            dummy_claims2.push(claim.clone());
        });

        dbg!("HI");

        let secp = Secp256k1::new();

        let mut rng = rand::thread_rng();

        let (_secret_key, public_key) = secp.generate_keypair(&mut rng);

        let mut hasher = DefaultHasher::new();
        public_key.hash(&mut hasher);
        let pubkey_hash = hasher.finish();

        let mut pub_key_bytes = pubkey_hash.to_string().as_bytes().to_vec();
        pub_key_bytes.push(1u8);

        let hash = digest(digest(&*pub_key_bytes).as_bytes());

        let payload = (10, 10, hash);

        if let Ok(seed1) = Quorum::generate_seed(payload.clone()) {
            if let Ok(seed2) = Quorum::generate_seed(payload.clone()) {
                if let Ok(mut quorum1) = Quorum::new(seed1, 11, 11) {
                    if let Ok(mut quorum2) = Quorum::new(seed2, 11, 11) {
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

