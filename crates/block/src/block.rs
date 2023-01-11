// This file contains code for creating blocks to be proposed, including the
// genesis block and blocks being mined.

use primitives::types::{
    Epoch, RawSignature, SerializedSecretKey as SecretKeyBytes, GENESIS_EPOCH, SECOND,
    VALIDATOR_THRESHOLD,
};
#[cfg(mainnet)]
use reward::reward::GENESIS_REWARD;
use reward::reward::{Reward, NUMBER_OF_BLOCKS_PER_EPOCH};
use ritelinked::LinkedHashMap;
use serde::{Deserialize, Serialize};
use sha256::{digest, digest_bytes};
use state::state::NetworkState;
use std::fmt;
use vrrb_core::accountable::Accountable;
use vrrb_core::claim::Claim;
use vrrb_core::txn::Txn;
use vrrb_core::verifiable::Verifiable;
use vrrb_core::keypair::KeyPair;

#[cfg(mainnet)]
use crate::genesis;
use crate::{
    header::BlockHeader,
    invalid::{InvalidBlockError, InvalidBlockErrorReason},
};

pub const GROSS_UTILITY_PERCENTAGE: f64 = 0.01;
pub const PERCENTAGE_CHANGE_SUPPLY_CAP: f64 = 0.25;

pub type CurrentUtility = i128;
pub type NextEpochAdjustment = i128;

pub struct MineArgs<'a> {
    /// The claim entitling the miner to mine the block.
    pub claim: Claim,
    pub last_block: Block,
    /// The last block, which contains the current block reward.
    pub txns: LinkedHashMap<String, Txn>,
    pub claims: LinkedHashMap<String, Claim>,
    pub claim_map_hash: Option<String>,
    pub reward: &'a mut Reward,
    pub network_state: &'a NetworkState,
    pub neighbors: Option<Vec<BlockHeader>>,
    pub abandoned_claim: Option<Claim>,
    pub secret_key: SecretKeyBytes,
    pub epoch: Epoch,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(C)]
pub struct Block {
    pub header: BlockHeader,
    pub neighbors: Option<Vec<BlockHeader>>,
    pub height: u128,
    // TODO: replace with Tx Trie Root
    pub txns: LinkedHashMap<String, Txn>,
    // TODO: Replace with Claim Trie Root
    pub claims: LinkedHashMap<String, Claim>,
    pub hash: Vec<u8>,
    pub received_at: Option<u128>,
    pub received_from: Option<String>,
    // TODO: Replace with map of all abandoned claims in the even more than 1 miner is faulty when
    // they are entitled to mine
    pub abandoned_claim: Option<Claim>,

    /// Quorum signature needed for finalizing the block and locking the chain
    pub threshold_signature: Option<RawSignature>,

    /// Epoch for which block was created
    pub epoch: Epoch,

    /// Measurement of utility for the chain
    pub utility: CurrentUtility,

    /// Adjustment For Next Epoch
    pub adjustment_for_next_epoch: Option<NextEpochAdjustment>,
}

impl Block {
    // Returns a result with either a tuple containing the genesis block and the
    // updated account state (if successful) or an error (if unsuccessful)
    pub fn genesis(claim: Claim, secret_key: Vec<u8>, miner: Option<String>) -> Result<Block, InvalidBlockErrorReason> {

        // Create the genesis header
        let header =
            BlockHeader::genesis(0, claim.clone(), secret_key, miner, RawSignature::default())?;
        // Create the genesis state hash
        // TODO: Replace with state trie root
        let mut state_hash = "".to_string();
        if let Ok(str_last_hash) = String::from_utf8(header.clone().last_hash) {
            state_hash = digest(
                format!(
                    "{},{}",
                    str_last_hash,
                    digest("Genesis_State_Hash".as_bytes())
                )
                .as_bytes(),
            );
        } else {
            return Err(InvalidBlockErrorReason::InvalidBlockHeader);
        }

        // Replace with claim trie
        let mut claims = LinkedHashMap::new();
        claims.insert(claim.clone().public_key, claim);

        #[cfg(mainnet)]
        let txns = genesis::generate_genesis_txns();

        // TODO: Genesis block on local/testnet should generate either a faucet for
        // tokens, or fill some initial accounts so that testing can be executed

        #[cfg(not(mainnet))]
        let txns = LinkedHashMap::new();
        let header = header.clone();

        let genesis = Block {
            header,
            neighbors: None,
            height: 0,
            txns,
            claims,
            hash: state_hash.as_bytes().to_vec(),
            received_at: None,
            received_from: None,
            abandoned_claim: None,
            threshold_signature: Some(vec![0; 5]),
            utility: 0,
            epoch: GENESIS_EPOCH,
            adjustment_for_next_epoch: None,
        };

        // Update the State Trie & Tx Trie with the miner and new block, this will also
        // set the values to the network state. Unwrap the result and assign it
        // to the variable updated_account_state to be returned by this method.
        Ok(genesis)
    }

    /// The mine method is used to generate a new block (and an updated account
    /// state with the reward set to the miner wallet's balance), this will
    /// also update the network state with a new confirmed state.
    pub fn mine(
        args: MineArgs,
        // claim: Claim,      // The claim entitling the miner to mine the block.
        // last_block: Block, // The last block, which contains the current block reward.
        // txns: LinkedHashMap<String, Txn>,
        // claims: LinkedHashMap<String, Claim>,
        // claim_map_hash: Option<String>,
        // reward: &mut Reward,
        // network_state: &NetworkState,
        // neighbors: Option<Vec<BlockHeader>>,
        // abandoned_claim: Option<Claim>,
        // signature: String,
        // epoch: Epoch,
    ) -> Result<(Option<Block>, NextEpochAdjustment), InvalidBlockErrorReason> {
        let claim = args.claim;
        let last_block = args.last_block;
        let txns = args.txns;
        let claims = args.claims;
        let claim_map_hash = args.claim_map_hash;
        let reward = args.reward;
        let network_state = args.network_state;
        let neighbors = args.neighbors;
        let abandoned_claim = args.abandoned_claim;
        let secret_key = args.secret_key;
        let epoch = args.epoch;

        // TODO: Replace with Tx Trie Root
        let txn_hash = {
            let mut txn_vec = vec![];
            txns.iter().for_each(|(_, v)| {
                txn_vec.extend(v.as_bytes());
            });
            digest(&*txn_vec)
        };

        // TODO: Remove there should be no neighbors
        let neighbors_hash = {
            let mut neighbors_vec = vec![];
            if let Some(neighbors) = &neighbors {
                neighbors.iter().for_each(|v| {
                    neighbors_vec.extend(v.as_bytes());
                });
                Some(digest(&*neighbors_vec))
            } else {
                None
            }
        };

        let utility_amount: i128 = txns.iter().map(|x| x.1.get_amount() as i128).sum();
        let mut adjustment_next_epoch = 0;
        let block_utility = if epoch != last_block.epoch {
            adjustment_next_epoch =
                Self::set_next_adjustment_epoch(&last_block, reward, utility_amount);
            utility_amount
        } else {
            utility_amount + last_block.utility
        };

        // TODO: Fix after replacing neighbors and tx hash/claim hash with respective
        // Trie Roots
        let header = BlockHeader::new(
            last_block.clone(),
            reward,
            claim,
            txn_hash,
            claim_map_hash,
            neighbors_hash,
            secret_key,
            epoch == last_block.epoch,
            adjustment_next_epoch,
            Some(vec![0; 5]),
        )?;

        // guaranteeing at least 1 second between blocks or whether some other
        // mechanism may serve the purpose better, or whether simply sequencing proposed
        // blocks and allowing validator network to determine how much time
        // between blocks has passed.

        if let Some(time) = header
            .clone()
            .timestamp
            .checked_sub(last_block.header.timestamp)
        {
            if (time / SECOND) < 1 {
                return Ok((None, 0i128));
            }
        } else {
            return Ok((None, 0i128));
        }

        let height = last_block.height + 1;
        let adjustment_next_epoch_opt = if adjustment_next_epoch != 0 {
            Some(adjustment_next_epoch)
        } else {
            None
        };

        let mut block = Block {
            header: header.clone(),
            neighbors,
            height,
            txns,
            claims,
            hash: header.last_hash,
            received_at: None,
            received_from: None,
            abandoned_claim,
            threshold_signature: Some(vec![0; 5]),
            utility: block_utility,
            epoch,
            adjustment_for_next_epoch: adjustment_next_epoch_opt,
        };

        // TODO: Replace with state trie
        let mut hashable_state = network_state.clone();

        let hash = digest(hashable_state.hash(&block.txns, block.header.block_reward.clone()))
            .as_bytes()
            .to_vec();
        block.hash = hash;
        return Ok((Some(block), adjustment_next_epoch));
    }

    /// If the utility amount is greater than the last block's utility, then the
    /// next adjustment epoch is the utility amount times the gross utility
    /// percentage. Otherwise, the next adjustment epoch is the utility
    /// amount times the negative gross utility percentage
    ///
    /// Arguments:
    ///
    /// * `last_block`: The last block in the chain.
    /// * `reward`: The reward for the current epoch.
    /// * `utility_amount`: The amount of utility that was generated in the last
    ///   epoch.
    ///
    /// Returns:
    ///
    /// The amount of the adjustment for the next epoch.
    fn set_next_adjustment_epoch(
        last_block: &Block,
        reward: &Reward,
        utility_amount: i128,
    ) -> i128 {
        let mut adjustment_next_epoch = if utility_amount > last_block.utility {
            (utility_amount as f64 * GROSS_UTILITY_PERCENTAGE) as i128
        } else {
            (utility_amount as f64 * -GROSS_UTILITY_PERCENTAGE) as i128
        };
        if let Some(adjustment_percentage_previous_epoch) = last_block.adjustment_for_next_epoch {
            if (adjustment_next_epoch / NUMBER_OF_BLOCKS_PER_EPOCH as i128)
                >= adjustment_percentage_previous_epoch * reward.amount as i128
            {
                adjustment_next_epoch = adjustment_percentage_previous_epoch
                    * (reward.amount * NUMBER_OF_BLOCKS_PER_EPOCH) as i128
            };
        };
        adjustment_next_epoch
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_string().as_bytes().to_vec()
    }

    pub fn from_bytes(data: &[u8]) -> Result<Block, InvalidBlockErrorReason> {
        let mut buffer: Vec<u8> = vec![];

        data.iter().for_each(|x| buffer.push(*x));

        if let Ok(to_string) = String::from_utf8(buffer) {
            if let Ok(block) = serde_json::from_str::<Block>(&to_string) {
                return Ok(block);
            } else {
                return Err(InvalidBlockErrorReason::General);
            }
        } else {
            return Err(InvalidBlockErrorReason::General);
        }
    }

    // TODO: Consider renaming to `serialize_to_string`
    #[allow(clippy::inherent_to_string_shadow_display)]
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Block(\n \
            header: {:?},\n",
            self.header
        )
    }
}

// TODO: Rewrite Verifiable to comport with Masternode Quorum Validation
// Protocol
impl Verifiable for Block {
    type Dependencies = NetworkState;
    type Error = InvalidBlockError;
    type Item = Block;

    fn verifiable(&self) -> bool {
        true
    }

    fn valid(
        &self,
        item: &Self::Item,
        dependencies: &Self::Dependencies,
    ) -> Result<bool, Self::Error> {
        if self.header.block_height > item.header.block_height + 1 {
            return Err(Self::Error::new(
                InvalidBlockErrorReason::BlockOutOfSequence,
            ));
        }

        if self.header.block_height <= item.header.block_height {
            return Err(Self::Error::new(InvalidBlockErrorReason::NotTallestChain));
        }

        if self.header.block_seed != item.header.next_block_seed {
            return Err(Self::Error::new(InvalidBlockErrorReason::InvalidBlockNonce));
        }

        if self.header.block_reward.get_amount() != item.header.next_block_reward.get_amount() {
            return Err(Self::Error::new(
                InvalidBlockErrorReason::InvalidBlockReward,
            ));
        }

        if let Some((hash, pointers)) =
            dependencies.get_lowest_pointer(self.header.block_seed as u128)
        {
            if hash == self.header.claim.hash {
                if let Some(claim_pointer) = self
                    .header
                    .claim
                    .get_pointer(self.header.block_seed as u128)
                {
                    if pointers != claim_pointer {
                        return Err(Self::Error::new(
                            InvalidBlockErrorReason::InvalidClaimPointers,
                        ));
                    }
                } else {
                    return Err(Self::Error::new(
                        InvalidBlockErrorReason::InvalidClaimPointers,
                    ));
                }
            } else {
                return Err(Self::Error::new(
                    InvalidBlockErrorReason::InvalidClaimPointers,
                ));
            }
        }

        if self.header.last_hash != item.hash {
            return Err(Self::Error::new(InvalidBlockErrorReason::InvalidLastHash));
        }

        if self.header.claim.valid(&None, &(None, None)).is_err() {
            return Err(Self::Error::new(InvalidBlockErrorReason::InvalidClaim));
        }

        Ok(true)
    }

    fn valid_genesis(&self, _dependencies: &Self::Dependencies) -> Result<bool, Self::Error> {
        let genesis_last_hash = digest("Genesis_Last_Hash".as_bytes()).as_bytes().to_vec();

        let mut genesis_state_hash = "".to_string();

        if let Ok(str_genesis_last_hash) = String::from_utf8(genesis_last_hash.clone()) {
            genesis_state_hash = digest(
                format!(
                    "{},{}",
                    str_genesis_last_hash,
                    digest("Genesis_State_Hash".as_bytes())
                )
                .as_bytes(),
            );
        } else {
            return Err(Self::Error::new(InvalidBlockErrorReason::InvalidLastHash));
        }

        if self.header.block_height != 0 {
            return Err(Self::Error::new(
                InvalidBlockErrorReason::InvalidBlockHeight,
            ));
        }

        if self.header.last_hash != genesis_last_hash {
            return Err(Self::Error::new(InvalidBlockErrorReason::InvalidLastHash));
        }

        if self.hash != genesis_state_hash.into_bytes() {
            return Err(Self::Error::new(InvalidBlockErrorReason::InvalidStateHash));
        }

        if self.header.claim.valid(&None, &(None, None)).is_err() {
            return Err(Self::Error::new(InvalidBlockErrorReason::InvalidClaim));
        }

        if KeyPair::verify_ecdsa_sign(
            self.header.signature.clone(),
            self.header.get_payload().as_bytes(),
            self.header.claim.public_key.as_bytes().to_vec(),
        )
        .is_err()
        {
            return Err(Self::Error::new(
                InvalidBlockErrorReason::InvalidBlockSignature,
            ));
        }

        let mut valid_data = true;
        self.txns.iter().for_each(|(_, txn)| {
            let n_valid = txn.validators().iter().filter(|(_, &valid)| valid).count();
            if (n_valid as f64 / txn.validators().len() as f64) < VALIDATOR_THRESHOLD {
                valid_data = false;
            }
        });

        if !valid_data {
            return Err(Self::Error::new(InvalidBlockErrorReason::InvalidTxns));
        }

        Ok(true)
    }
}
