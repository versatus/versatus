pub mod block;
pub mod genesis;
pub mod header;
pub mod invalid;
pub use crate::block::*;

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, time::UNIX_EPOCH};

    use rand::Rng;
    use reward::reward::Reward;
    use ritelinked::LinkedHashMap;
    use state::NetworkState;
    use vrrb_core::{claim::Claim, keypair::KeyPair, txn::Txn};

    use crate::{header::BlockHeader, Block, MineArgs};

    #[test]
    fn test_genesis_block_utility() {
        let keypair = KeyPair::random();
        let claim = Claim::new(keypair.miner_kp.1.to_string(), "address".to_string(), 1);
        let genesis_block_opt =
            Block::genesis(claim, keypair.miner_kp.0.secret_bytes().to_vec(), None);
        assert!(genesis_block_opt.is_some());
        let genesis_block = genesis_block_opt.unwrap();
        assert!(genesis_block.utility == 0);
    }

    #[test]
    fn test_block_utility() {
        let keypair = KeyPair::random();
        let claim = Claim::new(keypair.miner_kp.1.to_string(), "address".to_string(), 1);
        let genesis_block_opt =
            Block::genesis(claim, keypair.miner_kp.0.secret_bytes().to_vec(), None);
        let keypair_1 = KeyPair::random();
        let keypair_2 = KeyPair::random();
        let last_block_claim = Claim::new(keypair.miner_kp.1.to_string(), "address".to_string(), 2);
        let mut genesis_block = genesis_block_opt.unwrap();
        let start = std::time::SystemTime::now();
        let start = start
            .checked_sub(std::time::Duration::from_secs(5))
            .unwrap();
        let timestamp = start.duration_since(UNIX_EPOCH).unwrap().as_nanos();
        genesis_block.header.timestamp = timestamp;

        let mut reward = Reward::genesis(Some("MINER_1".to_string()));
        let block_headers = vec![get_block_header(2), get_block_header(3)];

        let mine_args = MineArgs {
            claim: last_block_claim.clone(),
            last_block: genesis_block,
            txns: get_txns(),
            claims: get_claims(),
            claim_map_hash: None,
            reward: &mut reward,
            network_state: &NetworkState::default(),
            neighbors: Some(block_headers.clone()),
            abandoned_claim: None,
            secret_key: keypair_1.miner_kp.0.secret_bytes().to_vec(),
            epoch: 1,
        };
        let last_block = Block::mine(mine_args);
        let start = start
            .checked_sub(std::time::Duration::from_secs(3))
            .unwrap();
        let timestamp = start.duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let mut last_block = last_block.0.unwrap();
        last_block.header.timestamp = timestamp;
        let new_block_claim = Claim::new(keypair.miner_kp.1.to_string(), "address".to_string(), 2);
        let mine_args = MineArgs {
            claim: new_block_claim.clone(),
            last_block: last_block.clone(),
            txns: get_txns(),
            claims: get_claims(),
            claim_map_hash: None,
            reward: &mut reward,
            network_state: &NetworkState::default(),
            neighbors: Some(block_headers.clone()),
            abandoned_claim: None,
            secret_key: keypair_2.miner_kp.0.secret_bytes().to_vec(),
            epoch: 1,
        };

        let block = Block::mine(mine_args);

        assert!((block.0.unwrap().utility + last_block.utility) > 0);
    }

    #[test]
    fn test_block_adjustment_reward() {
        let keypair = KeyPair::random();
        let claim = Claim::new(keypair.miner_kp.1.to_string(), "address".to_string(), 1);
        let genesis_block_opt =
            Block::genesis(claim, keypair.miner_kp.0.secret_bytes().to_vec(), None);
        let keypair_1 = KeyPair::random();
        let keypair_2 = KeyPair::random();
        let last_block_claim = Claim::new(keypair.miner_kp.1.to_string(), "address".to_string(), 2);

        let mut genesis_block = genesis_block_opt.unwrap();
        let start = std::time::SystemTime::now();
        let start = start
            .checked_sub(std::time::Duration::from_secs(5))
            .unwrap();
        let timestamp = start.duration_since(UNIX_EPOCH).unwrap().as_nanos();
        genesis_block.header.timestamp = timestamp;
        let mut reward = Reward::genesis(Some("MINER_1".to_string()));

        let block_headers = vec![get_block_header(2), get_block_header(3)];

        let mine_args = MineArgs {
            claim: last_block_claim,
            last_block: genesis_block,
            txns: get_txns(),
            claims: get_claims(),
            claim_map_hash: None,
            reward: &mut reward,
            network_state: &NetworkState::default(),
            neighbors: Some(block_headers),
            abandoned_claim: None,
            secret_key: keypair_1.miner_kp.0.secret_bytes().to_vec(),
            epoch: 0,
        };

        let last_block = Block::mine(mine_args);

        let start = start
            .checked_sub(std::time::Duration::from_secs(3))
            .unwrap();
        let timestamp = start.duration_since(UNIX_EPOCH).unwrap().as_nanos();

        let adjustment = last_block.1;
        reward.new_epoch(adjustment);

        assert!(reward.valid_reward());

        let mut last_block = last_block.0.unwrap();
        last_block.header.timestamp = timestamp;
        last_block.utility = 10;

        let new_block_claim = Claim::new("pubkey".to_string(), "address".to_string(), 2);

        let mine_args = MineArgs {
            claim: new_block_claim,
            last_block,
            txns: get_txns(),
            claims: get_claims(),
            claim_map_hash: None,
            reward: &mut reward,
            network_state: &NetworkState::default(),
            neighbors: None,
            abandoned_claim: None,
            secret_key: keypair_2.miner_kp.0.secret_bytes().to_vec(),
            epoch: 1,
        };

        let block = Block::mine(mine_args);
        let adjustment_next_epoch = block.1;
        let block_data = block.0.unwrap();

        assert_eq!(
            adjustment_next_epoch,
            ((block_data.utility as f64) * 0.01) as i128
        );

        assert!(block_data.adjustment_for_next_epoch.is_some());

        if let Some(adjustment_for_next_epoch) = block_data.adjustment_for_next_epoch {
            assert!(adjustment_for_next_epoch > 0);
        }

        assert!(reward.valid_reward());
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
                public_key: "".to_string(),
                address: "".to_string(),
                hash: "".to_string(),
                nonce: 0,
                eligible: false,
            },
            claim_map_hash: None,
            block_reward: Reward {
                miner: None,
                amount: 0,
                epoch: 1,
                next_epoch_block: 1,
                current_block: 1,
            },
            next_block_reward: Reward {
                miner: None,
                amount: 100,
                epoch: 1,
                next_epoch_block: 1,
                current_block: 2,
            },
            neighbor_hash: None,
            signature: "".to_string(),
        }
    }

    pub fn get_txns() -> LinkedHashMap<String, Txn> {
        let mut txns = LinkedHashMap::new();
        let mut rng = rand::thread_rng();
        let random_number = rng.gen_range(1, 10);
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
                    sender_public_key: String::from("ABC_PUB").as_bytes().to_vec(),
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
                    public_key: "PUB_KEY".to_string(),
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
