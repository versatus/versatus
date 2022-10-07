use serde::{Deserialize, Serialize};

use crate::types::DkgError;

/// `ThresholdConfig` is a struct that contains two fields, `upper_bound` and
/// `threshold`, both of which are unsigned 16-bit integers.
///
/// Properties:
///
/// * `upper_bound`: The upper bound value for no of nodes in LLMQ.
/// * `threshold`: The value determines minimum no of nodes needed to sign the
///   message.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThresholdConfig {
    pub upper_bound: u16,
    pub threshold: u16,
}

impl ThresholdConfig {
    const MINIMUM_NODES: u16 = 2;

    pub fn validate(&self) -> Result<(), DkgError> {
        if self.upper_bound < ThresholdConfig::MINIMUM_NODES || self.upper_bound == u16::MAX {
            return Err(DkgError::ConfigInvalidValue(
                "DKG Threshold config upper bound".to_string(),
                format!(
                    "{} < {} or == MAX",
                    self.upper_bound.clone(),
                    ThresholdConfig::MINIMUM_NODES
                ),
            ));
        }
        if self.threshold > self.upper_bound || self.threshold == 0 || self.threshold == u16::MAX {
            return Err(DkgError::ConfigInvalidValue(
                "DKG Threshold".to_string(),
                format!(
                    " {} == 0 || > {} || == MAX",
                    self.threshold.clone(),
                    self.upper_bound.clone()
                ),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // use super::*;
    use primitives::is_enum_variant;

    use crate::{
        test_utils::{invalid_threshold_config, valid_threshold_config},
        types::DkgError,
    };

    #[test]
    fn successful_validate_invalid_threshold_config() {
        let invalid_config = invalid_threshold_config();
        let result = invalid_config.validate();
        assert!(is_enum_variant!(
            result,
            Err(DkgError::ConfigInvalidValue { .. })
        ));
    }

    #[test]
    fn successful_validate_valid_threshold_config() {
        let valid_config = valid_threshold_config();
        let result = valid_config.validate();
        assert!(is_enum_variant!(result, Ok(())));
    }
}
