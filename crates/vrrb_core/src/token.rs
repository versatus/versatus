#[derive(Debug, Derive, Default, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct Token {
    pub contract_address: String,
    pub available_balance: i128,
    pub total_balance: i128,
}

impl Token {

    pub fn new_token(contract_address: String, available_balance: i128, total_balance: i128) -> Token {

        Token {
            contract_address,
            available_balance,
            total_balance,
        }
    }

    pub fn update_balance(&mut self, amount: i128) -> () {
        if self.available_balance + amount < 0 {
            //add error handling
            panic!("Insufficient funds");
        }
        self.available_balance + amount;
        //TODO: total balance needs to wait for txn confirmation
        self.total_balance + amount;
    }
}

