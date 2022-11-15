pub mod mempool_processor;
pub mod txn_validator;
pub mod validator_unit;

#[cfg(test)]
mod tests {

    use std::{
        collections::HashSet,
        sync::{mpsc::channel, Arc},
        thread,
        time::Duration,
    };

    use lr_trie::LeftRightTrie;
    use mempool::mempool::LeftRightMemPoolDB;
    use patriecia::db::MemoryDB;
    use secp256k1::{PublicKey, Secp256k1, SecretKey};
    use txn::txn::*;

    use crate::{
        mempool_processor::{
            MempoolControlMsg,
            MempoolTxnProcessor,
            MempoolTxnProcessorError,
            MempoolTxnProcessorState,
        },
        validator_unit::ValidatorUnit,
    };


    #[test]
    fn new_validator_creates_properly() {
        let mempool_pending = LeftRightMemPoolDB::new();

        let memdb = Arc::new(MemoryDB::new(true));
        let state_rh_factory = LeftRightTrie::new(memdb).factory();

        let (mempool_processor_sender, _) = channel();
        let (cores_error_channel_s, _) = channel();

        let amount_of_cores = 10;
        let validator = ValidatorUnit::new(
            mempool_pending.read,
            state_rh_factory,
            mempool_processor_sender,
            amount_of_cores,
            cores_error_channel_s,
        );

        assert_eq!(validator.cores.len() as u8, amount_of_cores);
    }

    pub fn new_signed_txn() -> Transaction {
        let secp = Secp256k1::new();
        let mut rng = rand::thread_rng();
        let secret = SecretKey::new(&mut rng);
        let mut txn = Transaction {
            instructions: Default::default(),
            sender: PublicKey::from_secret_key(&secp, &secret),
            signature: Default::default(),
            receipt: Default::default(),
            priority: Default::default(),
        };

        txn.sign(&secret).unwrap();
        txn
    }

    #[test]
    fn new_mempool_processor_creates_properly() {
        let mempool = LeftRightMemPoolDB::new();
        let memdb = Arc::new(MemoryDB::new(true));
        let state_rh_factory = LeftRightTrie::new(memdb).factory();

        let amount_of_cores = 10;

        let (mempool_processor_sender, mempool_processor_receiver) = channel();

        let (cores_error_channel_s, _) = channel();
        let validator = ValidatorUnit::new(
            mempool.read.clone(),
            state_rh_factory,
            mempool_processor_sender,
            amount_of_cores,
            cores_error_channel_s,
        );

        let (mempool_error_s, _) = channel();
        let mempool_processor = MempoolTxnProcessor::new(
            mempool_processor_receiver,
            validator,
            mempool,
            mempool_error_s,
        );

        assert_eq!(mempool_processor.validator.cores.len(), 10);
    }

    #[test]
    fn verify_that_txns_are_validated_and_invalid_are_written_to_rejected_pool() {
        let mempool = LeftRightMemPoolDB::new();
        // let mempool_validated = LeftRightMemPoolDB::new();
        let mempool_read_handle = mempool.read.factory();

        let memdb = Arc::new(MemoryDB::new(true));
        let state_rh_factory = LeftRightTrie::new(memdb).factory();

        let amount_of_cores = 1;

        let (mempool_processor_sender, mempool_processor_receiver) = channel();

        let (core_error_s, _) = channel();
        let validator = ValidatorUnit::new(
            mempool.read.clone(),
            state_rh_factory,
            mempool_processor_sender,
            amount_of_cores,
            core_error_s,
        );

        let (mempool_error_s, _) = channel();
        let mut mempool_processor = MempoolTxnProcessor::new(
            mempool_processor_receiver,
            validator,
            mempool,
            mempool_error_s,
        );

        let mut new_txns = HashSet::new();

        for _ in 0..500 {
            // 500 valid txns
            new_txns.insert(new_signed_txn());
        }

        for _ in 0..500 {
            // 500 unsigned txns
            new_txns.insert(Transaction::default());
        }

        let mempool_processor_sender = mempool_processor.validator.mempool_processor_sender.clone();

        thread::spawn(move || mempool_processor.start());
        mempool_processor_sender
            .send(MempoolControlMsg::NewFromNetwork(new_txns))
            .unwrap();


        // Add timeout
        thread::sleep(Duration::from_secs(2));

        if let Some(map) = mempool_read_handle
            .handle()
            .enter()
            .map(|guard| guard.clone())
        {
            assert_eq!(
                (0, 500, 500),
                (map.pending.len(), map.rejected.len(), map.validated.len())
            )
        } else {
            panic!("Should've been able to acquire guard and check lengths");
        }
    }

    #[test]
    fn verify_that_invalid_control_msg_sequence_generates_error() {
        let mempool = LeftRightMemPoolDB::new();

        let memdb = Arc::new(MemoryDB::new(true));
        let state_rh_factory = LeftRightTrie::new(memdb).factory();

        let amount_of_cores = 10;

        let (mempool_processor_sender, mempool_processor_receiver) = channel();

        let (core_error_s, _) = channel();
        let validator = ValidatorUnit::new(
            mempool.read.clone(),
            state_rh_factory,
            mempool_processor_sender,
            amount_of_cores,
            core_error_s,
        );

        let (mempool_error_s, mempool_error_r) = channel();
        let mut mempool_processor = MempoolTxnProcessor::new(
            mempool_processor_receiver,
            validator,
            mempool,
            mempool_error_s,
        );

        let mut new_txns = HashSet::new();

        for _ in 0..1000 {
            new_txns.insert(Transaction::default());
        }

        let mempool_processor_sender = mempool_processor.validator.mempool_processor_sender.clone();

        thread::spawn(move || mempool_processor.start());
        mempool_processor_sender
            .send(MempoolControlMsg::NewFromNetwork(new_txns))
            .unwrap();

        thread::sleep(Duration::from_secs(3));

        let mut new_txns = HashSet::new();

        for _ in 0..1000 {
            new_txns.insert(Transaction::default());
        }

        mempool_processor_sender
            .send(MempoolControlMsg::Stop)
            .unwrap();
        mempool_processor_sender
            .send(MempoolControlMsg::NewFromNetwork(new_txns.clone()))
            .unwrap();

        thread::sleep(Duration::from_secs(1));
        let err = mempool_error_r.try_recv();


        assert_eq!(
            err.unwrap(),
            MempoolTxnProcessorError::InvalidMsgForCurrentState(
                MempoolControlMsg::NewFromNetwork(new_txns),
                MempoolTxnProcessorState::Inactive
            )
        )
    }
}
