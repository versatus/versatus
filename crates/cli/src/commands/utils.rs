use std::path::PathBuf;

use vrrb_config::NodeConfig;

use crate::result::{CliError, Result};

pub fn write_node_config_from_file(node_config: &NodeConfig) -> Result<()> {
    let node_config_dir = node_config.db_path();
    let node_config_path = node_config_dir.join("config.json");

    let node_config_json = serde_json::to_string_pretty(&node_config)
        .map_err(|err| CliError::Other(format!("unable to serialize node config: {err}")))?;

    std::fs::write(node_config_path, node_config_json)
        .map_err(|err| CliError::Other(format!("unable to write node config file: {err}")))?;

    Ok(())
}

pub fn read_node_config_from_file(config_file_path: PathBuf) -> Result<NodeConfig> {
    let path_str = config_file_path.to_str().unwrap_or_default();

    let node_config = NodeConfig::from_file(path_str)
        .map_err(|err| CliError::Other(format!("failed to read config file: {err}")))?;

    Ok(node_config)
}

// TODO: fix state I/O && test writing txns to state
