use crate::block::Block;
use crate::blockchain::{InvalidBlockError, InvalidBlockErrorReason};
use crate::pool::Pool;
use crate::reward::RewardState;
use crate::state::NetworkState;
use crate::txn::Txn;

pub trait Verifiable {
    fn verifiable(&self) -> bool;

    fn valid_block(
        &self,
        _last_block: &Block,
        _network_state: &NetworkState,
        _reward_state: &RewardState,
    ) -> Result<(), InvalidBlockError> {
        Err(InvalidBlockError {
            details: InvalidBlockErrorReason::General,
        })
    }

    fn valid_genesis(&self, _network_state: &NetworkState, _reward_state: &RewardState) -> bool {
        false
    }

    fn valid_last_hash(&self, _last_block: &Block) -> bool {
        false
    }

    fn valid_state_hash(&self, _network_state: &NetworkState) -> bool {
        false
    }

    fn valid_block_reward(&self, _reward_state: &RewardState) -> bool {
        false
    }

    fn valid_next_block_reward(&self, _reward_state: &RewardState) -> bool {
        false
    }

    fn valid_txns(&self) -> bool {
        false
    }

    fn valid_block_nonce(&self, _last_block: &Block) -> bool {
        false
    }

    fn valid_claim_pointer(&self, _network_state: &NetworkState) -> bool {
        false
    }

    fn valid_block_claim(&self, _network_state: &NetworkState) -> bool {
        false
    }

    fn valid_block_signature(&self) -> bool {
        false
    }

    fn valid_txn(&self, _network_state: &NetworkState, _txn_pool: &Pool<String, Txn>) -> bool {
        false
    }

    fn valid_txn_signature(&self) -> bool {
        false
    }

    fn valid_amount(&self, _network_state: &NetworkState, _txn_pool: &Pool<String, Txn>) -> bool {
        false
    }

    fn check_double_spend(&self, _txn_pool: &Pool<String, Txn>) -> bool {
        false
    }

    fn check_txn_nonce(&self, _network_state: &NetworkState) -> bool {
        false
    }
}
