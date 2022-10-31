use ritelinked::LinkedHashMap;

struct VestingConfig {
    pub cliff_fraction: f64,
    pub cliff_years: f64,
    pub unlocks: usize,
    pub unlock_years: f64,
}

// 50% after one year, then monthly for 12 months
const EMPLOYEE_VESTING: VestingConfig = VestingConfig {
    cliff_fraction: 0.5,
    cliff_years: 1,
    unlocks: 12,
    unlock_years: 1,
};

// 25% after half year, then monthly for 18  months
const INVESTOR_VESTING: VestingConfig = VestingConfig {
    cliff_fraction: 0.25,
    cliff_years: 0.75,
    unlocks: 18,
    unlock_years: 1.5,
};
pub fn generate_txns() -> LinkedHashMap<String, Txn> {}
