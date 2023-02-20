//FEATURE TAG(S): Rewards, Block Structure

use serde::{Deserialize, Serialize};
use vrrb_core::accountable::Accountable;

// UNITS
pub const SPECK: u128 = 1;
pub const TRIXIMO: u128 = 1000 * SPECK;
pub const NIFADA: u128 = 1000 * TRIXIMO;
pub const RIMA: u128 = 1000 * NIFADA;
pub const SITARI: u128 = 1000 * RIMA;
pub const PSIGMA: u128 = 1000 * SITARI;
pub const VRRB: u128 = 1000 * PSIGMA;

// Generate a random variable reward to include in new blocks

pub const MAX_REWARD_ADJUSTMENT: f32 = 0.25;
pub const BASELINE_REWARD: u128 = 20;
pub const MIN_BASELINE_REWARD: u128 = 15;
pub const MAX_BASELINE_REWARD: u128 = 25;
pub const NUMBER_OF_BLOCKS_PER_EPOCH: u128 = 30000000;
pub const GENESIS_REWARD: u128 = 400_000_000;

/// `Reward` is a struct that contains the epoch, next epoch block, current
/// block, miner, and amount.
///
/// Properties:
///
/// * `epoch`: The epoch number
/// * `next_epoch_block`: The block number of the next epoch.
/// * `current_block`: The current block number.
/// * `miner`: The address of the miner who mined the block.
/// * `amount`: The amount of tokens that will be rewarded to the miner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct Reward {
    pub epoch: u128,
    pub next_epoch_block: u128,
    pub current_block: u128,
    pub miner: Option<String>,
    pub amount: u128,
}

impl Reward {
    /// `start` function for Genesis Reward
    ///
    /// Arguments:
    ///
    /// * `miner`: The address of the miner who mined the block.
    ///
    /// Returns:
    ///
    /// A Reward struct
    pub fn genesis(miner: Option<String>) -> Reward {
        Reward {
            current_block: 0,
            epoch: 1,
            next_epoch_block: NUMBER_OF_BLOCKS_PER_EPOCH,
            miner,
            amount: BASELINE_REWARD,
        }
    }

    pub fn generate_next_reward(&self, adjustment_to_next_epoch: i128) -> Reward {
        let rem = (self.current_block + 1) % NUMBER_OF_BLOCKS_PER_EPOCH;
        if rem == 0 {
            let nr_epoch = self.epoch + 1;
            let nr_next_epoch_block = self.next_epoch_block + NUMBER_OF_BLOCKS_PER_EPOCH;
            let mut nr_amount = {
                (self.amount as i128
                    + (adjustment_to_next_epoch / NUMBER_OF_BLOCKS_PER_EPOCH as i128))
                    as u128
            };

            if nr_amount < MIN_BASELINE_REWARD {
                nr_amount = MIN_BASELINE_REWARD;
            } else if nr_amount > MAX_BASELINE_REWARD {
                nr_amount = MAX_BASELINE_REWARD;
            }

            Reward {
                current_block: self.current_block,
                epoch: nr_epoch,
                next_epoch_block: nr_next_epoch_block,
                miner: None,
                amount: nr_amount,
            }
        } else {
            self.clone()
        }
    }

    #[deprecated(note = "replaced by generate_next_reward method")]
    pub fn update(&mut self, adjustment_to_next_epoch: i128) {
        self.new_epoch(adjustment_to_next_epoch);
    }

    /// This function resets the amount of reward to the baseline reward
    pub fn reset(&mut self) {
        self.amount = BASELINE_REWARD;
    }

    /// The function `new_epoch` is called when a new epoch is reached. It
    /// increments the epoch number, calculates the next epoch block, and
    /// adjusts the block reward
    ///
    /// Arguments:
    ///
    /// * `adjustment_to_next_epoch`: The amount of adjustment to the next
    ///   epoch.
    #[deprecated(note = "deprecated as a result of self.update() method being deprecated")]
    pub fn new_epoch(&mut self, adjustment_to_next_epoch: i128) {
        self.epoch += 1;

        // Next nth Block for epoch to end
        self.next_epoch_block += NUMBER_OF_BLOCKS_PER_EPOCH;

        //Adjust Block Reward
        let amount =
            self.amount as i128 + adjustment_to_next_epoch / NUMBER_OF_BLOCKS_PER_EPOCH as i128;
        if amount <= 0 {
            self.amount = MIN_BASELINE_REWARD;
        } else if amount >= MAX_BASELINE_REWARD as i128 {
            self.amount = MAX_BASELINE_REWARD;
        } else {
            self.amount = amount as u128;
        }
    }

    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let as_string = serde_json::to_string(self).unwrap();
        as_string.as_bytes().to_vec()
    }

    pub fn from_bytes(data: &[u8]) -> Reward {
        let mut buffer: Vec<u8> = vec![];

        data.iter().for_each(|x| buffer.push(*x));

        let to_string = String::from_utf8(buffer).unwrap();

        serde_json::from_str::<Reward>(&to_string).unwrap()
    }

    /// > This function checks if the reward is within the range of the minimum
    /// > and maximum baseline
    /// reward
    ///
    /// Returns:
    ///
    /// A boolean value.
    pub fn valid_reward(&self) -> bool {
        self.amount >= MIN_BASELINE_REWARD || self.amount <= MAX_BASELINE_REWARD
    }
}

impl Default for Reward {
    fn default() -> Reward {
        Reward {
            current_block: 0,
            epoch: 0,
            next_epoch_block: NUMBER_OF_BLOCKS_PER_EPOCH,
            miner: None,
            amount: BASELINE_REWARD,
        }
    }
}

impl Accountable for Reward {
    type Category = String;

    fn receivable(&self) -> String {
        self.miner.clone().unwrap()
    }

    fn payable(&self) -> Option<String> {
        None
    }

    fn get_amount(&self) -> u128 {
        self.amount
    }

    fn get_category(&self) -> Option<Self::Category> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{Reward, BASELINE_REWARD};
    use crate::reward::{MAX_BASELINE_REWARD, MIN_BASELINE_REWARD, NUMBER_OF_BLOCKS_PER_EPOCH};

    #[test]
    fn test_reward_state_starting_point() {
        let reward = Reward::genesis(Some("MINER_1".to_string()));
        assert!(reward.amount == BASELINE_REWARD);
        assert!(reward.epoch == 1);
        assert!(reward.next_epoch_block == NUMBER_OF_BLOCKS_PER_EPOCH);
    }

    #[test]
    fn test_reward_state_after_next_epoch() {
        let mut reward = Reward::genesis(Some("MINER_1".to_string()));
        reward.new_epoch(15);
        assert!(reward.amount >= MIN_BASELINE_REWARD && reward.amount <= MAX_BASELINE_REWARD);
    }

    #[test]
    fn test_restored_reward_state() {
        let mut reward = Reward::genesis(Some("MINER".to_string()));
        reward.new_epoch(15);
        assert!(reward.amount >= MIN_BASELINE_REWARD && reward.amount <= MAX_BASELINE_REWARD);
        assert!(reward.valid_reward());
        reward.reset();
        assert!(reward.amount == BASELINE_REWARD);
    }
}
