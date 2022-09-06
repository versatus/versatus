pub mod mempool_processor;
pub mod txn_validator;
pub mod validator_unit;

#[cfg(test)]
mod tests {
    //     use std::sync::mpsc::{channel, Receiver, Sender};
    //     use std::{collections::HashMap, sync::Arc, thread, time::Duration};

    //     use lr_trie::db::MemoryDB;
    //     use mempool::mempool::*;

    use std::{
        collections::{HashMap, HashSet},
        sync::{mpsc::channel, Arc},
        thread,
        time::Duration,
    };

    use mempool::mempool::LeftRightMemPoolDB;
    use patriecia::db::MemoryDB;
    use rand::{rngs::StdRng, Rng, SeedableRng};
    use state_trie::StateTrie;
    //     use crate::validator_unit::*;
    //     use rand::rngs::StdRng;
    //     use rand::{self, Rng, SeedableRng};
    //     use state_trie::StateTrie;
    use txn::txn::*;

    use crate::{
        mempool_processor::{
            MempoolControlMsg, MempoolTxnProcessor, MempoolTxnProcessorError,
            MempoolTxnProcessorState,
        },
        validator_unit::ValidatorUnit,
    };

    //     // TODO: Use proper txns when there will be proper txn validation implemented
    fn random_string(rng: &mut StdRng) -> String {
        format!("{}", rng.gen::<u32>())
    }

    fn random_txn(rng: &mut StdRng) -> Txn {
        Txn {
            txn_id: random_string(rng),
            txn_timestamp: 0,
            sender_address: random_string(rng),
            sender_public_key: random_string(rng),
            receiver_address: random_string(rng),
            txn_token: None,
            txn_amount: 0,
            txn_payload: random_string(rng),
            txn_signature: random_string(rng),
            validators: HashMap::<String, bool>::new(),
            nonce: 0,
        }
    }

    #[test]
    fn new_validator_creates_properly() {
        let mempool_pending = LeftRightMemPoolDB::new();

        let memdb = Arc::new(MemoryDB::new(true));
        let _state = StateTrie::new(memdb).factory();

        let (mempool_processor_sender, _) = channel();
        let (cores_error_channel_s, _) = channel();

        let amount_of_cores = 10;
        let validator = ValidatorUnit::new(
            mempool_pending.read,
            _state,
            mempool_processor_sender,
            amount_of_cores,
            cores_error_channel_s,
        );

        assert_eq!(validator.cores.len() as u8, amount_of_cores);
    }
    #[test]
    fn new_mempool_processor_creates_properly() {
        let mempool_pending = LeftRightMemPoolDB::new();
        let mempool_validated = LeftRightMemPoolDB::new();
        let memdb = Arc::new(MemoryDB::new(true));
        let _state = StateTrie::new(memdb).factory();

        let amount_of_cores = 10;

        let (mempool_processor_sender, mempool_processor_receiver) = channel();

        let (cores_error_channel_s, _) = channel();
        let validator = ValidatorUnit::new(
            mempool_pending.read.clone(),
            _state,
            mempool_processor_sender,
            amount_of_cores,
            cores_error_channel_s,
        );

        let (mempool_error_s, _) = channel();
        let mempool_processor = MempoolTxnProcessor::new(
            mempool_processor_receiver,
            validator,
            mempool_pending,
            mempool_error_s,
        );

        assert_eq!(mempool_processor.validator.cores.len(), 10);
    }

    #[test]
    fn verify_that_txns_are_validated_and_invalid_are_written_to_rejected_pool() {
        let mempool_pending = LeftRightMemPoolDB::new();
        let mempool_validated = LeftRightMemPoolDB::new();
        let mempool_validated_read_handle = mempool_validated.read.clone();

        let memdb = Arc::new(MemoryDB::new(true));
        let _state = StateTrie::new(memdb).factory();

        let amount_of_cores = 10;

        let (mempool_processor_sender, mempool_processor_receiver) = channel();

        let (core_error_s, _) = channel();
        let validator = ValidatorUnit::new(
            mempool_pending.read.clone(),
            _state,
            mempool_processor_sender,
            amount_of_cores,
            core_error_s,
        );

        let (mempool_error_s, _) = channel();
        let mut mempool_processor = MempoolTxnProcessor::new(
            mempool_processor_receiver,
            validator,
            mempool_pending,
            mempool_error_s,
        );

        let mut new_txns = HashSet::new();

        let mut rng = rand::rngs::StdRng::from_seed([0; 32]);
        for _ in 0..1000 {
            new_txns.insert(random_txn(&mut rng));
        }

        let mempool_processor_sender = mempool_processor.validator.mempool_processor_sender.clone();

        thread::spawn(move || mempool_processor.start());
        mempool_processor_sender
            .send(MempoolControlMsg::NewFromNetwork(new_txns))
            .unwrap();

        thread::sleep(Duration::from_secs(3));

        assert_eq!(
            mempool_validated_read_handle
                .enter()
                .unwrap()
                .rejected
                .len(),
            1000
        );
    }

    #[test]
    fn verify_that_invalid_control_msg_sequence_generates_error() {
        let mempool_pending = LeftRightMemPoolDB::new();
        let mempool_validated = LeftRightMemPoolDB::new();
        let mempool_validated_read_handle = mempool_validated.read.clone();

        let memdb = Arc::new(MemoryDB::new(true));
        let _state = StateTrie::new(memdb).factory();

        let amount_of_cores = 10;

        let (mempool_processor_sender, mempool_processor_receiver) = channel();

        let (core_error_s, _) = channel();
        let validator = ValidatorUnit::new(
            mempool_pending.read.clone(),
            _state,
            mempool_processor_sender,
            amount_of_cores,
            core_error_s,
        );

        let (mempool_error_s, mempool_error_r) = channel();
        let mut mempool_processor = MempoolTxnProcessor::new(
            mempool_processor_receiver,
            validator,
            mempool_pending,
            mempool_error_s,
        );

        let mut new_txns = HashSet::new();

        let mut rng = rand::rngs::StdRng::from_seed([0; 32]);
        for _ in 0..1000 {
            new_txns.insert(random_txn(&mut rng));
        }

        let mempool_processor_sender = mempool_processor.validator.mempool_processor_sender.clone();

        thread::spawn(move || mempool_processor.start());
        mempool_processor_sender
            .send(MempoolControlMsg::NewFromNetwork(new_txns))
            .unwrap();

        thread::sleep(Duration::from_secs(3));

        let mut new_txns = HashSet::new();

        let mut rng = rand::rngs::StdRng::from_seed([0; 32]);
        for _ in 0..1000 {
            new_txns.insert(random_txn(&mut rng));
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
            mempool_validated_read_handle
                .enter()
                .unwrap()
                .validated
                .len(),
            1000
        );
        assert_eq!(
            err.unwrap(),
            MempoolTxnProcessorError::InvalidMsgForCurrentState(
                MempoolControlMsg::NewFromNetwork(new_txns),
                MempoolTxnProcessorState::Inactive
            )
        )
    }
}
