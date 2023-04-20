use std::{fmt::Display, str::FromStr};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Environment {
    #[default]
    Local,
    Testnet,
    Mainnet,
}

pub const VRRB_ENVIRONMENT_VAR_NAME: &str = "VRRB_ENVIRONMENT";
pub const VRRB_PRETTY_PRINT_LOGS_VAR_NAME: &str = "VRRB_PRETTY_PRINT_LOGS";

pub fn get_vrrb_environment() -> Environment {
    std::env::var(VRRB_ENVIRONMENT_VAR_NAME)
        .unwrap_or(Environment::default().to_string())
        .parse()
        .unwrap_or(Environment::default())
}

pub fn get_pretty_print_logs() -> bool {
    std::env::var(VRRB_PRETTY_PRINT_LOGS_VAR_NAME)
        .unwrap_or("false".to_string())
        .parse()
        .unwrap_or(false)
}

pub fn set_pretty_print_logs() {
    std::env::set_var(VRRB_PRETTY_PRINT_LOGS_VAR_NAME, "true");
}

impl Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Environment::Local => write!(f, "local"),
            Environment::Testnet => write!(f, "testnet"),
            Environment::Mainnet => write!(f, "mainnet"),
        }
    }
}

impl FromStr for Environment {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "local" | "dev" => Ok(Environment::Local),
            "testnet" | "test" | "stg" => Ok(Environment::Testnet),
            "mainnet" | "main" | "prod" => Ok(Environment::Mainnet),
            _ => Err(crate::Error::InvalidEnvironment(s.to_string())),
        }
    }
}
