use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::{env, fs};

// NOTE: this is used to generate random filenames so files created by tests don't get overwritten
fn generate_random_string() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(30)
        .map(char::from)
        .collect()
}

use state::NodeState;

#[test]
fn can_be_serialized_into_a_json_file() {
    let temp_dir_path = env::temp_dir();
    let state_backup_path = temp_dir_path.join(format!("{}.json", generate_random_string()));

    fs::write(&state_backup_path, b"").unwrap();

    let node_state = NodeState::new(state_backup_path.clone());

    node_state.serialize_to_json().unwrap();

    let read_data = fs::read_to_string(state_backup_path).unwrap();

    assert!(!read_data.is_empty());
}

#[test]
fn can_be_restored_from_json_file() {
    let temp_dir_path = env::temp_dir();
    let state_backup_path = temp_dir_path.join(format!("{}.json", generate_random_string()));

    let node_state = NodeState::new(state_backup_path.clone());

    node_state.serialize_to_json().unwrap();

    // let restored_node_state = NodeState::restore(&state_backup_path).unwrap();
    // assert!(!restored_node_state.is_empty());
    let node_state = NodeState::restore(&state_backup_path).unwrap();
    node_state.values();
}

#[test]
fn should_not_restore_state_from_invalid_paths() {
    let temp_dir_path = env::temp_dir();
    let state_backup_path = temp_dir_path.join(format!("{}", generate_random_string()));

    let node_state = NodeState::new(state_backup_path.clone());

    node_state.serialize_to_json().unwrap();

    let restored_node_state = NodeState::restore(&state_backup_path);

    assert!(restored_node_state.is_err());
}

#[test]
fn should_not_restore_state_from_malformed_data() {
    let temp_dir_path = env::temp_dir();
    let state_backup_path = temp_dir_path.join(format!("{}.json", generate_random_string()));

    let restored_node_state = NodeState::restore(&state_backup_path);

    assert!(restored_node_state.is_err());
}
