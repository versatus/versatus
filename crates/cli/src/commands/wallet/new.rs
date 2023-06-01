use std::path::Path;

use secp256k1::{generate_keypair, rand};
use vrrb_core::helpers::write_keypair_file;
use wallet::v2::{AddressAlias, Wallet};

use crate::result::CliError;

pub async fn exec(wallet: &mut Wallet, path: &Path, alias: AddressAlias) -> Result<(), CliError> {
    // TODO: read keypair from file

    let (secret_key, public_key) = generate_keypair(&mut rand::thread_rng());

    let account_data_dir = path.join(format!("{alias}"));

    std::fs::create_dir_all(&account_data_dir)?;

    let key_path = account_data_dir.join("keys");
    let account_path = account_data_dir.join("account.json");

    let (_, account) = wallet
        .create_account(alias, public_key)
        .await
        .map_err(|err| CliError::Other(format!("unable to create account in state: {err}")))?;

    write_keypair_file(key_path, &(secret_key, public_key))
        .map_err(|err| CliError::Other(format!("unable to write keypair file: {err}")))?;

    let account_ser = serde_json::to_string_pretty(&account)
        .map_err(|err| CliError::Other(format!("unable to serialize account data: {err}")))?;

    std::fs::write(account_path, account_ser)
        .map_err(|err| CliError::Other(format!("unable to write account file: {err}")))?;

    Ok(())
}
