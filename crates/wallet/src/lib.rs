pub mod wallet;
pub mod cli_args;

#[cfg(test)]
mod tests {
    use crate::cli_args::WalletArgs;
    use clap::Parser;
    #[test]
    fn basic_cli_testing() {
        let args = WalletArgs::parse_from(&["wallet", "arg_one", "arg_two"]);
    }
}
