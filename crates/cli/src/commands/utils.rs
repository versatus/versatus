// TODO: fix state I/O && test writing txns to state

use primitives::{KademliaPeerId, NodeId};
use serde_json::{from_str as json_from_str, from_value as json_from_value, Value as JsonValue};
use std::path::PathBuf;
use utils::payload::digest_data_to_bytes;
use vrrb_config::QuorumMember;

use crate::result::{CliError, Result};

// TODO: split reading the file and deserializing into two functions
pub fn deserialize_whitelisted_quorum_members(
    whitelist: String,
    finalized_whitelist: &mut Vec<QuorumMember>,
) -> Result<()> {
    let whitelist_path = PathBuf::from(whitelist);
    let whitelist_str =
        std::fs::read_to_string(whitelist_path).map_err(|e| CliError::OptsError(e.to_string()))?;
    let whitelist_values: JsonValue =
        json_from_str(&whitelist_str).map_err(|e| CliError::OptsError(e.to_string()))?;

    if let JsonValue::Object(whitelist_members) = whitelist_values {
        for (node_type, value) in whitelist_members {
            match node_type.as_str() {
                "genesis-miner" => finalized_whitelist
                    .push(json_from_value(value).map_err(|e| CliError::OptsError(e.to_string()))?),
                "genesis-farmers" | "genesis-harvesters" => {
                    if let JsonValue::Array(genesis_quorum_members) = value {
                        for member in genesis_quorum_members {
                            finalized_whitelist.push(
                                json_from_value(member)
                                    .map_err(|e| CliError::OptsError(e.to_string()))?,
                            )
                        }
                    }
                },
                _ => {
                    return Err(CliError::OptsError(
                        "invalid genesis node type found in whitelist config".to_string(),
                    ));
                },
            }
        }
    }
    Ok(())
}

pub fn derive_kademlia_peer_id_from_node_id(
    node_id: &NodeId,
) -> crate::result::Result<KademliaPeerId> {
    // NOTE: turns a node's id into a 32 byte array
    let node_key_bytes = digest_data_to_bytes(node_id);

    let kademlia_key = kademlia_dht::Key::try_from(node_key_bytes).map_err(|err| {
        CliError::Other(format!(
            "Failed to convert node key to Kademlia key: expected 32 byte length, got error: {}",
            err
        ))
    })?;

    Ok(kademlia_key)
}
