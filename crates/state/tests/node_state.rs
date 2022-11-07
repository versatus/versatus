use std::{env, fs};

use rand::{distributions::Alphanumeric, thread_rng, Rng};

// NOTE: this is used to generate random filenames so files created by tests
// don't get overwritten
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
fn accounts_can_be_added() {
    let temp_dir_path = env::temp_dir();
    let state_backup_path = temp_dir_path.join(format!("{}.json", generate_random_string()));

    let mut node_state = NodeState::new(state_backup_path.clone());

    node_state.add_account(
        b"my_mock_pkey".to_vec(),
        lrdb::Account {
            hash: String::from(""),
            nonce: 1234456,
            credits: 0,
            debits: 0,
            storage: None,
            code: None,
        },
    );

    node_state.add_account(
        b"my_mock_pkey_2".to_vec(),
        lrdb::Account {
            hash: String::from(""),
            nonce: 1234456,
            credits: 0,
            debits: 0,
            storage: None,
            code: None,
        },
    );

    node_state.serialize_to_json().unwrap();

    let entries = node_state.entries();

    assert_eq!(entries.len(), 2);

    node_state.extend_accounts(vec![
        (
            b"my_mock_pkey_3".to_vec(),
            lrdb::Account {
                hash: String::from(""),
                nonce: 1234456,
                credits: 0,
                debits: 0,
                storage: None,
                code: None,
            },
        ),
        (
            b"my_mock_pkey_4".to_vec(),
            lrdb::Account {
                hash: String::from(""),
                nonce: 1234456,
                credits: 0,
                debits: 0,
                storage: None,
                code: None,
            },
        ),
        (
            b"my_mock_pkey_5".to_vec(),
            lrdb::Account {
                hash: String::from(""),
                nonce: 1234456,
                credits: 0,
                debits: 0,
                storage: None,
                code: None,
            },
        ),
    ]);

    let entries = node_state.entries();

    assert_eq!(entries.len(), 5);
}

#[test]
fn accounts_can_be_retrieved() {
    let temp_dir_path = env::temp_dir();
    let state_backup_path = temp_dir_path.join(format!("{}.json", generate_random_string()));

    let mut node_state = NodeState::new(state_backup_path.clone());

    node_state.add_account(
        b"my_mock_pkey".to_vec(),
        lrdb::Account {
            hash: String::from(""),
            nonce: 1234456,
            credits: 0,
            debits: 0,
            storage: None,
            code: None,
        },
    );

    node_state.add_account(
        b"my_mock_pkey_2".to_vec(),
        lrdb::Account {
            hash: String::from(""),
            nonce: 1234456,
            credits: 0,
            debits: 0,
            storage: None,
            code: None,
        },
    );

    node_state.serialize_to_json().unwrap();

    node_state.get_account(&b"my_mock_pkey".to_vec()).unwrap();
    node_state.get_account(&b"my_mock_pkey_2".to_vec()).unwrap();
}

#[test]
fn can_be_restored_from_json_file() {
    let temp_dir_path = env::temp_dir();
    let state_backup_path = temp_dir_path.join(format!("{}.json", generate_random_string()));

    let node_state = NodeState::new(state_backup_path.clone());
    node_state.serialize_to_json().unwrap();

    NodeState::restore(&state_backup_path).unwrap();
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
