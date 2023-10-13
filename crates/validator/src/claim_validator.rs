use std::result::Result as StdResult;

use hbbft::crypto::{PublicKey, Signature};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use vrrb_core::{
    claim::{Claim, Eligibility},
    staking::{Stake, StakeUpdate, MIN_STAKE_FARMER, MIN_STAKE_VALIDATOR},
};

pub type Result<T> = StdResult<T, ClaimValidatorError>;

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq, Hash)]
pub enum ClaimValidatorError {
    #[error("Non eligible claim")]
    NotEligibleClaim,

    #[error("Non enough stake for {0}")]
    NotEnoughStake(String),

    #[error("Invalid Stake Txn")]
    InvalidStakeTxn,

    #[error("Non Certified Stake ")]
    NonCertifiedStake,

    #[error("timestamp {0} is outside of the permitted date range [0, {1}]")]
    OutOfBoundsTimestamp(i64, i64),

    #[error("value {0} is outside of the permitted range [{1}, {2}]")]
    OutOfBounds(String, String, String),

    #[error("Quorum Key missing")]
    QuorumKeyMissing,

    #[error("Invalid Quorum Key ")]
    InvalidQuorumKey,

    #[error("One of the stake of claim has invalid Certificate")]
    InvalidStakeCertificate,

    #[error("Temporary Jailed, Node needs to put stake to get unjailed")]
    Jailed,
}

#[derive(Debug, Clone)]
pub struct ClaimValidator;

impl ClaimValidator {
    /// The function validates a claim by checking if it is eligible, has enough
    /// stake, and verifying the stake transactions and certificates.
    ///
    /// Arguments:
    ///
    /// * `claim`: The `claim` parameter is a reference to a `Claim` object that
    ///   needs to be validated.
    ///
    /// Returns:
    ///
    /// a `Result<()>` type, which is an empty tuple `()` wrapped in a `Result`
    /// enum. If the validation is successful, it returns an `Ok(())`
    /// variant, and if there is an error, it returns an `Err` variant with
    /// a `ClaimValidatorError` enum as the error type.
    pub fn validate(&self, claim: &Claim) -> Result<()> {
        match claim.eligibility {
            Eligibility::Validator => {
                if claim.get_stake() < MIN_STAKE_VALIDATOR {
                    return Err(ClaimValidatorError::NotEnoughStake(
                        claim.eligibility.to_string(),
                    ));
                }
            },
            Eligibility::None => {
                return Err(ClaimValidatorError::NotEligibleClaim);
            },
            Eligibility::Miner => {},
        }

        let stakes = claim.get_stake_txns();
        if let Some(last_stake) = stakes.last() {
            if let StakeUpdate::Slash(amount) = last_stake.get_amount() {
                if amount > 0 {
                    return Err(ClaimValidatorError::Jailed);
                }
            }
        }

        stakes
            .par_iter()
            .try_for_each(|stake: &Stake| -> Result<()> {
                stake
                    .verify()
                    .map_err(|_| ClaimValidatorError::InvalidStakeTxn)?;
                self.validate_timestamp(stake)?;
                match stake.get_certificate() {
                    None => return Err(ClaimValidatorError::NonCertifiedStake),
                    Some(certificate) => {
                        if stake.get_quorum_key().is_empty() {
                            return Err(ClaimValidatorError::QuorumKeyMissing);
                        }
                        let public_key_arr = TryInto::<[u8; 48]>::try_into(stake.get_quorum_key())
                            .map_err(|_| ClaimValidatorError::InvalidQuorumKey)?;
                        let public_key = PublicKey::from_bytes(public_key_arr)
                            .map_err(|_| ClaimValidatorError::InvalidQuorumKey)?;

                        let signature_arr = TryInto::<[u8; 96]>::try_into(certificate.0.as_slice())
                            .map_err(|_| ClaimValidatorError::InvalidStakeCertificate)?;

                        let signature = Signature::from_bytes(signature_arr)
                            .map_err(|_| ClaimValidatorError::InvalidStakeCertificate)?;

                        let status = public_key.verify(&signature, certificate.1);
                        if !status {
                            return Err(ClaimValidatorError::InvalidStakeCertificate);
                        }
                    },
                }

                Ok(())
            })?;

        Ok(())
    }

    pub fn validate_timestamp(&self, stake: &Stake) -> Result<()> {
        let timestamp = chrono::offset::Utc::now().timestamp();
        let stake_timestamp = stake.get_timestamp();
        if stake_timestamp > 0 && stake_timestamp < timestamp {
            Ok(())
        } else {
            Err(ClaimValidatorError::OutOfBoundsTimestamp(
                stake_timestamp,
                timestamp,
            ))
        }
    }
}
