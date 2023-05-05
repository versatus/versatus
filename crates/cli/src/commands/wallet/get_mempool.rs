use crate::result::CliError;

pub(crate) async fn exec(
    wallet: &mut wallet::v2::Wallet,
    limit: Option<usize>,
) -> crate::result::Result<()> {
    let result = wallet.get_mempool().await?;

    let ser_result =
        serde_json::to_string_pretty(&result).map_err(|e| CliError::Other(e.to_string()))?;

    println!("{ser_result}");

    let displayable_limit = limit.unwrap_or(0);
    println!("{displayable_limit}");

    Ok(())
}
