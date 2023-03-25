use crate::{
    commands::utils::read_node_config_from_file,
    result::{CliError, Result},
};

pub async fn exec() -> Result<()> {
    let node_config = read_node_config_from_file()
        .map_err(|err| CliError::Other(format!("unable to read node config: {err}")))?;

    let node_config = serde_json::to_string_pretty(&node_config)
        .map_err(|err| CliError::Other(format!("unable to serialize node config: {err}")))?;

    println!("{}", node_config);

    Ok(())
}
