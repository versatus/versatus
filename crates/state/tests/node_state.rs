use std::{env, fs, path::PathBuf};

use state::NodeState;

#[test]
fn can_be_serialized_into_a_json_file() {
    let temp_dir_path = env::temp_dir();
    let mut state_backup_path = temp_dir_path.clone().join("state.json");

    fs::write(&state_backup_path, b"").unwrap();

    let node_state = NodeState::new(state_backup_path.clone());

    node_state.serialize_to_json().unwrap();

    let read_data = fs::read_to_string(state_backup_path.clone()).unwrap();

    assert!(read_data.len() > 0);
}

#[test]
fn can_be_restored_from_json_file() {
    let temp_dir_path = env::temp_dir();
    let mut state_backup_path = temp_dir_path.clone().join("state.json");

    let node_state = NodeState::new(state_backup_path);

    node_state.serialize_to_json();
}
