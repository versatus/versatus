use crate::ThresholdConfig;

pub fn valid_threshold_config() -> ThresholdConfig {
    ThresholdConfig {
        upper_bound: 4,
        threshold: 1,
    }
}

pub fn invalid_threshold_config() -> ThresholdConfig {
    ThresholdConfig {
        upper_bound: 4,
        threshold: 5,
    }
}
