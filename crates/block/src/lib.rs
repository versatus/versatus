pub mod block;
pub mod genesis;
pub mod header;
pub mod invalid;
pub use crate::block::*;

#[cfg(test)]
mod tests {
    use std::time::UNIX_EPOCH;

    use claim::claim::Claim;
    use lr_trie::LeftRightTrie;
    use mempool::mempool::{LeftRightMemPoolDB, TxnStatus};
    use patriecia::db::MemoryDB;
    use primitives::types::{
        rand::{thread_rng, Rng},
        PublicKey,
        Secp256k1,
        SecretKey,
    };
    use reward::reward::{Reward, RewardState};
    use ritelinked::LinkedHashMap;
    use txn::txn::{NativeToken, SystemInstruction, Transaction, TransferData};

    use crate::{header::BlockHeader, Block};

    #[test]
    fn test_genesis_block_utility() {
        let secp = Secp256k1::new();
        let (secret_key, _) = secp.generate_keypair(&mut thread_rng());
        let claim = Claim::new("pubkey".to_string(), "address".to_string(), 1);
        let genesis_block_opt = Block::genesis(&reward_state, claim, secret_key);
        assert!(genesis_block_opt.is_some());
        let genesis_block = genesis_block_opt.unwrap();
        assert!(genesis_block.utility == 0);
    }

    #[test]
    fn test_block_utility() {
        let secp = Secp256k1::new();
        let (secret_key, _) = secp.generate_keypair(&mut thread_rng());
        let claim = Claim::new("pubkey".to_string(), "address".to_string(), 1);
        let genesis_block_opt = Block::genesis(&reward_state, claim, secret_key);
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

        let txns = get_txns();

        let mut mempool = LeftRightMemPoolDB::new();

        for (_, txn) in txns {
            mempool.add_txn(&txn, TxnStatus::Validated).unwrap();
        }

        let block_headers = vec![get_block_header(2), get_block_header(3)];
        let last_block = Block::mine::<MemoryDB>(
            &last_block_claim,
            genesis_block,
            &mempool,
            &get_claims(),
            None,
            &RewardState::start(),
            &LeftRightTrie::default(),
            &Some(block_headers),
            None,
            secret_key_1,
            1,
        );
        let start = start
            .checked_sub(std::time::Duration::from_secs(3))
            .unwrap();
        let timestamp = start.duration_since(UNIX_EPOCH).unwrap().as_nanos();

        let mut last_block = last_block.0.unwrap();
        last_block.header.timestamp = timestamp;

        let new_block_claim = Claim::new("pubkey".to_string(), "address".to_string(), 2);
        let txns = get_txns();

        let mut mempool = LeftRightMemPoolDB::new();

        for (_, txn) in txns {
            mempool.add_txn(&txn, TxnStatus::Validated).unwrap();
        }
        let block = Block::mine::<MemoryDB>(
            &new_block_claim,
            last_block.clone(),
            &mempool,
            &get_claims(),
            None,
            &RewardState::start(),
            &LeftRightTrie::default(),
            &None,
            None,
            secret_key_2,
            1,
        );


        assert!((block.0.unwrap().utility + last_block.utility) > 0);
    }

    #[test]
    fn test_block_adjustment_reward() {
        let secp = Secp256k1::new();
        let (secret_key, _) = secp.generate_keypair(&mut thread_rng());
        let claim = Claim::new("pubkey".to_string(), "address".to_string(), 1);
        let genesis_block_opt = Block::genesis(&reward_state, claim, secret_key);
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
        let mut reward = Reward::genesis(Some("MINER_1".to_string()));

        let block_headers = vec![get_block_header(2), get_block_header(3)];
        let txns = get_txns();

        let mut mempool = LeftRightMemPoolDB::new();

        for (_, txn) in txns {
            mempool.add_txn(&txn, TxnStatus::Validated).unwrap();
        }
        let last_block = Block::mine::<MemoryDB>(
            &last_block_claim,
            genesis_block,
            &mempool,
            &get_claims(),
            None,
            &RewardState::start(),
            &LeftRightTrie::default(),
            &Some(block_headers),
            None,
            secret_key_1,
            0,
        );
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
        let txns = get_txns();

        let mut mempool = LeftRightMemPoolDB::new();

        for (_, txn) in txns {
            mempool.add_txn(&txn, TxnStatus::Validated).unwrap();
        }
        let new_block_claim = Claim::new("pubkey".to_string(), "address".to_string(), 2);
        let block = Block::mine::<MemoryDB>(
            &new_block_claim,
            last_block.clone(),
            &mempool,
            &get_claims(),
            None,
            &RewardState::start(),
            &LeftRightTrie::default(),
            &None,
            None,
            secret_key_2,
            1,
        );
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


    pub fn new_random_signed_txn() -> Transaction {
        let secp = Secp256k1::new();
        let mut rng = primitives::types::rand::thread_rng();
        let amount: u128 = rng.gen();
        let secret = SecretKey::new(&mut rng);
        let pubkey = PublicKey::from_secret_key(&secp, &secret);
        let mut txn = Transaction {
            instructions: vec![SystemInstruction::Transfer(TransferData {
                from: pubkey,
                to: pubkey,
                amount: NativeToken(1 + amount % 200u128),
            })],
            sender: pubkey,
            signature: Default::default(),
            receipt: Default::default(),
            priority: Default::default(),
        };

        txn.sign(&secret).unwrap();
        txn
    }
    #[allow(deprecated)]
    pub fn get_txns() -> LinkedHashMap<String, Transaction> {
        let mut txns = LinkedHashMap::new();
        let mut rng = thread_rng();
        let random_number = 5 + rng.gen::<i32>() % 10;

        for _ in 0..random_number {
            let tx = new_random_signed_txn();

            txns.insert(tx.get_id(), tx);
        }
        txns
    }

    pub fn get_claims() -> LinkedHashMap<String, Claim> {
        let mut claims = LinkedHashMap::new();
        let mut rng = thread_rng();
        let random_number = rng.gen::<i32>() % 10;
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
