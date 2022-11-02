use ritelinked::LinkedHashMap;
use txn::txn::Txn;
use vesting::VestingConfig;

// 50% after one year, then monthly for 12 months
const EMPLOYEE_VESTING: VestingConfig = VestingConfig {
    cliff_fraction: 0.5f64,
    cliff_years: 1f64,
    unlocks: 12,
    unlock_years: 1f64,
};

// 25% after half year, then monthly for 18  months
const INVESTOR_VESTING: VestingConfig = VestingConfig {
    cliff_fraction: 0.25f64,
    cliff_years: 0.75f64,
    unlocks: 18,
    unlock_years: 1.5f64,
};

const EMPLOYEESS: [&str; 2] = [
    "6pME5t7fLGubJjcn4L4ncN7DZs7tWPHU4FoYV4Cv3vGa",
    "3ki3QpPM2cGE3X5MZm6L4NcMdrp7R9vVeE2PEE427uqa",
];


const INVESTORS: [&str; 2] = [
    "C5dz418Wf5cKeGKCUBN7AUTRcGK9wcRknEfVbjSyAMZm",
    "5TAgthC5PLYBP3JSvjjfnd1jkY1VLrC78p4T4MucHFE",
];

// TODO: replace that mock module, when vesting contract is written
mod vesting {
    use txn::txn::Txn;

    pub struct VestingConfig {
        pub cliff_fraction: f64,
        pub cliff_years: f64,
        pub unlocks: usize,
        pub unlock_years: f64,
    }

    pub fn create_vesting(target: &str, config: VestingConfig) -> (String, Txn) {
        return (
            String::new(),
            Txn {
                txn_id: todo!(),
                txn_timestamp: todo!(),
                sender_address: todo!(),
                sender_public_key: todo!(),
                receiver_address: todo!(),
                txn_token: todo!(),
                txn_amount: todo!(),
                txn_payload: todo!(),
                txn_signature: todo!(),
                validators: todo!(),
                nonce: todo!(),
            },
        );
    }
}

pub fn generate_genesis_txns() -> LinkedHashMap<String, Txn> {
    let mut genesis_txns: LinkedHashMap<String, Txn> = LinkedHashMap::new();
    for employee in EMPLOYEESS {
        let vesting_txn = vesting::create_vesting(employee, EMPLOYEE_VESTING);
        genesis_txns.insert(vesting_txn.0, vesting_txn.1);
    }
    for investor in INVESTORS {
        let vesting_txn = vesting::create_vesting(investor, INVESTOR_VESTING);
        genesis_txns.insert(vesting_txn.0, vesting_txn.1);
    }
    genesis_txns
}
