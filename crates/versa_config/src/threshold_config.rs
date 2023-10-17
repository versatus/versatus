use serde::{Deserialize, Serialize};

use crate::ConfigError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Eq)]
pub struct ThresholdConfig {
    pub upper_bound: u16,
    pub threshold: u16,
}

impl Default for ThresholdConfig {
    fn default() -> Self {
        ThresholdConfig {
            upper_bound: 4,
            threshold: 2,
        }
    }
}

impl ThresholdConfig {
    const MINIMUM_NODES: u16 = 2;

    pub fn validate(&self) -> crate::Result<()> {
        if self.upper_bound < ThresholdConfig::MINIMUM_NODES || self.upper_bound == u16::MAX {
            return Err(ConfigError::Other(format!(
                "DKG Threshold config upper bound {} < {} or == MAX",
                self.upper_bound.clone(),
                ThresholdConfig::MINIMUM_NODES
            )));
        }
        if self.threshold > self.upper_bound || self.threshold == 0 || self.threshold == u16::MAX {
            return Err(ConfigError::Other(format!(
                "DKG threshold {} == 0 || > {} || == MAX",
                self.threshold.clone(),
                self.upper_bound.clone()
            )));
        }
        Ok(())
    }
}
