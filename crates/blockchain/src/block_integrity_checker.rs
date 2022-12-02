use block::Block;
use primitives::types::VALIDATOR_THRESHOLD;

pub struct BlockIntegrityChecker {}

pub type Result<T> = std::result::Result<T, BlockIntegrityError>;

#[derive(Debug, thiserror::Error)]
pub enum BlockIntegrityError {
    #[error("block contains invalid transactions")]
    InvalidTxns,
    #[error("invalid claim")]
    InvalidClaim,
    #[error("invalid last hash")]
    InvalidLastHash,
    #[error("invalid state hash")]
    InvalidStateHash,
    #[error("invalid block height")]
    InvalidBlockHeight,
    #[error("invalid block nonce")]
    InvalidBlockNonce,
    #[error("invalid block reward")]
    InvalidBlockReward,
    #[error("invalid claim pointers")]
    InvalidClaimPointers,
    #[error("invalid next block reward")]
    InvalidNextBlockReward,
    #[error("invalid block signature")]
    InvalidBlockSignature,
}

impl BlockIntegrityChecker {
    pub fn is_valid_genesis_block(&self, block: &Block) -> Result<bool> {
        let genesis_last_hash = digest("Genesis_Last_Hash".as_bytes());
        let genesis_state_hash = digest(
            format!(
                "{},{}",
                genesis_last_hash,
                digest("Genesis_State_Hash".as_bytes())
            )
            .as_bytes(),
        );

        if block.header.block_height != 0 {
            return Err(BlockIntegrityError::InvalidBlockHeight);
        }

        if block.header.last_hash != genesis_last_hash {
            return Err(BlockIntegrityError::InvalidLastHash);
        }

        if block.hash != genesis_state_hash {
            return Err(BlockIntegrityError::InvalidStateHash);
        }

        if block.header.claim.valid(&None, &(None, None)).is_err() {
            return Err(BlockIntegrityError::InvalidClaim);
        }

        if block.header.verify().is_err() {
            return Err(BlockIntegrityError::InvalidBlockSignature);
        }

        let mut valid_data = true;
        block.txns.iter().for_each(|(_, txn)| {
            let n_valid = txn.validators.iter().filter(|(_, &valid)| valid).count();
            if (n_valid as f64 / txn.validators.len() as f64) < VALIDATOR_THRESHOLD {
                valid_data = false;
            }
        });

        if !valid_data {
            return Err(BlockIntegrityError::InvalidTxns);
        }

        Ok(true)
    }

    pub fn is_valid_block(
        &self,
        item: &Self::Item,
        dependencies: &Self::Dependencies,
    ) -> Result<bool> {
        if self.header.block_height > item.header.block_height + 1 {
            return Err(Self::Error::new(
                InvalidBlockErrorReason::BlockOutOfSequence,
            ));
        }

        if self.header.block_height <= item.header.block_height {
            return Err(Self::Error::new(InvalidBlockErrorReason::NotTallestChain));
        }

        if self.header.block_nonce != item.header.next_block_nonce {
            return Err(Self::Error::new(InvalidBlockErrorReason::InvalidBlockNonce));
        }

        if self.header.block_reward.get_amount() != item.header.next_block_reward.get_amount() {
            return Err(Self::Error::new(
                InvalidBlockErrorReason::InvalidBlockReward,
            ));
        }

        if let Some((hash, pointers)) =
            dependencies.get_lowest_pointer(self.header.block_nonce as u128)
        {
            if hash == self.header.claim.hash {
                if let Some(claim_pointer) = self
                    .header
                    .claim
                    .get_pointer(self.header.block_nonce as u128)
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
}
