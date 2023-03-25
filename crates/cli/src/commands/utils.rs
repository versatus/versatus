use std::{fs, path::PathBuf};

use primitives::DEFAULT_VRRB_DB_PATH;
use vrrb_config::NodeConfig;

use crate::result::{CliError, Result};

pub fn config_file_path() -> PathBuf {
    let node_config_path = format!("{}{}", DEFAULT_VRRB_DB_PATH, "/config.json");

    return PathBuf::from(node_config_path);
}

pub fn node_config_exists() -> Result<bool> {
    let node_config_path = config_file_path();

    let metadata = fs::metadata(&node_config_path)
        .map_err(|err| CliError::Other(format!("unable to see if config file exists: {err}")))?;

    if metadata.is_file() {
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn write_node_config_from_file(node_config: &NodeConfig) -> Result<()> {
    let node_config_path = config_file_path();

    let node_config_json = serde_json::to_string_pretty(&node_config)
        .map_err(|err| CliError::Other(format!("unable to serialize node config: {err}")))?;

    std::fs::write(node_config_path, node_config_json)
        .map_err(|err| CliError::Other(format!("unable to write node config file: {err}")))?;

    Ok(())
}

pub fn read_node_config_from_file() -> Result<NodeConfig> {
    let node_config_path = config_file_path();

    let node_config = NodeConfig::from_file(&node_config_path)
        .map_err(|err| CliError::Other(format!("failed to read config file: {err}")))?;

    Ok(node_config)
}

// TODO: fix state I/O && test writing txns to state
