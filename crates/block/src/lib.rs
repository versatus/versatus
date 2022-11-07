pub mod block;
pub mod header;
pub mod invalid;
pub use crate::block::*;

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, time::UNIX_EPOCH};

    use claim::claim::Claim;
    use rand::{thread_rng, Rng};
    use reward::reward::{Reward, RewardState};
    use ritelinked::LinkedHashMap;
    use secp256k1::Secp256k1;
    use state::NetworkState;
    use txn::txn::Txn;

    use crate::{header::BlockHeader, Block};

    #[test]
    fn test_genesis_block_utility() {
        let secp = Secp256k1::new();
        let (secret_key, _) = secp.generate_keypair(&mut thread_rng());
        let reward_state = RewardState::start();
        let claim = Claim::new("pubkey".to_string(), "address".to_string(), 1);
        let genesis_block_opt = Block::genesis(&reward_state, claim, secret_key.to_string());
        assert!(genesis_block_opt.is_some());
        let genesis_block = genesis_block_opt.unwrap();
        assert!(genesis_block.utility == 0);
    }

    #[test]
    fn test_block_utility() {
        let secp = Secp256k1::new();
        let (secret_key, _) = secp.generate_keypair(&mut thread_rng());
        let reward_state = RewardState::start();
        let claim = Claim::new("pubkey".to_string(), "address".to_string(), 1);
        let genesis_block_opt = Block::genesis(&reward_state, claim, secret_key.to_string());
        let (secret_key_1, _) = secp.generate_keypair(&mut thread_rng());

        let (secret_key_2, _) = secp.generate_keypair(&mut thread_rng());

        let last_block_claim = Claim::new("pubkey".to_string(), "address".to_string(), 2);

        let mut genesis_block = genesis_block_opt.unwrap();
        let start = std::time::SystemTime::now();
        let start = start
            .checked_sub(std::time::Duration::from_secs(5))
            .unwrap();
        let timestamp = start.duration_since(UNIX_EPOCH).unwrap().as_nanos();
        genesis_block.header.timestamp = timestamp;

        let block_headers = vec![get_block_header(2), get_block_header(3)];
        let last_block = Block::mine(
            last_block_claim,
            genesis_block,
            get_txns(),
            get_claims(),
            None,
            &RewardState::start(),
            &NetworkState::default(),
            Some(block_headers),
            None,
            secret_key_1.to_string(),
            1,
        );
        let start = start
            .checked_sub(std::time::Duration::from_secs(3))
            .unwrap();
        let timestamp = start.duration_since(UNIX_EPOCH).unwrap().as_nanos();

        let mut last_block = last_block.0.unwrap();
        last_block.header.timestamp = timestamp;

        let new_block_claim = Claim::new("pubkey".to_string(), "address".to_string(), 2);

        let block = Block::mine(
            new_block_claim,
            last_block.clone(),
            get_txns(),
            get_claims(),
            None,
            &RewardState::start(),
            &NetworkState::default(),
            None,
            None,
            secret_key_2.to_string(),
            1,
        );
        assert!((block.0.unwrap().utility + last_block.utility) > 0);
    }

    #[test]
    fn test_block_adjustment_reward() {
        let secp = Secp256k1::new();
        let (secret_key, _) = secp.generate_keypair(&mut thread_rng());
        let reward_state = RewardState::start();
        let claim = Claim::new("pubkey".to_string(), "address".to_string(), 1);
        let genesis_block_opt = Block::genesis(&reward_state, claim, secret_key.to_string());
        let (secret_key_1, _) = secp.generate_keypair(&mut thread_rng());

        let (secret_key_2, _) = secp.generate_keypair(&mut thread_rng());

        let last_block_claim = Claim::new("pubkey".to_string(), "address".to_string(), 2);

        let mut genesis_block = genesis_block_opt.unwrap();
        let start = std::time::SystemTime::now();
        let start = start
            .checked_sub(std::time::Duration::from_secs(5))
            .unwrap();
        let timestamp = start.duration_since(UNIX_EPOCH).unwrap().as_nanos();
        genesis_block.header.timestamp = timestamp;

        let block_headers = vec![get_block_header(2), get_block_header(3)];
        let last_block = Block::mine(
            last_block_claim,
            genesis_block,
            get_txns(),
            get_claims(),
            None,
            &RewardState::start(),
            &NetworkState::default(),
            Some(block_headers),
            None,
            secret_key_1.to_string(),
            0,
        );
        let start = start
            .checked_sub(std::time::Duration::from_secs(3))
            .unwrap();
        let timestamp = start.duration_since(UNIX_EPOCH).unwrap().as_nanos();

        let mut last_block = last_block.0.unwrap();
        last_block.header.timestamp = timestamp;
        last_block.utility = 10;

        let new_block_claim = Claim::new("pubkey".to_string(), "address".to_string(), 2);
        let block = Block::mine(
            new_block_claim,
            last_block.clone(),
            get_txns(),
            get_claims(),
            None,
            &RewardState::start(),
            &NetworkState::default(),
            None,
            None,
            secret_key_2.to_string(),
            1,
        );
        let adjustment_next_epoch = block.1;
        let block_data = block.0.unwrap();
        assert_eq!(
            adjustment_next_epoch,
            ((block_data.utility as f64) * 0.01) as u128
        );
    }

    pub fn get_block_header(seconds: u64) -> BlockHeader {
        let start = std::time::SystemTime::now();
        let start = start
            .checked_add(std::time::Duration::from_secs(seconds))
            .unwrap();
        let timestamp = start.duration_since(UNIX_EPOCH).unwrap().as_nanos();
        BlockHeader {
            last_hash: "".to_string(),
            block_nonce: 0,
            next_block_nonce: 0,
            block_height: 0,
            timestamp,
            txn_hash: "".to_string(),
            claim: Claim {
                pubkey: "".to_string(),
                address: "".to_string(),
                hash: "".to_string(),
                nonce: 0,
                eligible: false,
            },
            claim_map_hash: None,
            block_reward: Reward {
                miner: None,
                amount: 0,
            },
            next_block_reward: Reward {
                miner: None,
                amount: 0,
            },
            neighbor_hash: None,
            signature: "".to_string(),
        }
    }

    pub fn get_txns() -> LinkedHashMap<String, Txn> {
        let mut txns = LinkedHashMap::new();
        let mut rng = rand::thread_rng();
        let random_number = rng.gen_range(0, 10);
        let start = std::time::SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        for i in 0..random_number {
            let txn_amount: u128 = rng.gen_range(150, 1000);
            let nonce: u128 = rng.gen_range(10, 100);
            let time_stamp = (since_the_epoch.as_secs() * 1000
                + since_the_epoch.subsec_nanos() as u64 / 1_000_000)
                * 1000
                * 1000;
            txns.insert(
                i.to_string(),
                Txn {
                    txn_id: i.to_string(),
                    txn_timestamp: time_stamp as u128,
                    sender_address: String::from("ABC"),
                    sender_public_key: String::from("ABC_PUB"),
                    receiver_address: String::from("DEST"),
                    txn_token: None,
                    txn_amount,
                    txn_payload: String::from("sample_payload"),
                    txn_signature: String::from("signature"),
                    validators: HashMap::default(),
                    nonce,
                },
            );
        }
        txns
    }

    pub fn get_claims() -> LinkedHashMap<String, Claim> {
        let mut claims = LinkedHashMap::new();
        let mut rng = rand::thread_rng();
        let random_number = rng.gen_range(0, 10);
        for i in 0..random_number {
            claims.insert(
                i.to_string(),
                Claim {
                    pubkey: "PUB_KEY".to_string(),
                    address: "Address".to_string(),
                    hash: "Data".to_string(),
                    nonce: i as u128,
                    eligible: true,
                },
            );
        }
        claims
    }
}
