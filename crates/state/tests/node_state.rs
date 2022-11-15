use std::{env, fs};

use primitives::types::{PublicKey, SecretKey};
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

use secp256k1::Secp256k1;
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

    let mut rng = rand::thread_rng();
    let secp = Secp256k1::new();

    let mut keys: Vec<PublicKey> = vec![];

    for _ in 0..5 {
        let secret = SecretKey::new(&mut rng);
        keys.push(PublicKey::from_secret_key(&secp, &secret));
    }


    node_state.add_account(
        keys[0],
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
        keys[1],
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
            keys[2],
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
            keys[3],
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
            keys[4],
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

    let mut rng = rand::thread_rng();
    let secp = Secp256k1::new();
    let secret1 = SecretKey::new(&mut rng);
    let secret2 = SecretKey::new(&mut rng);
    let account1 = PublicKey::from_secret_key(&secp, &secret1);
    let account2 = PublicKey::from_secret_key(&secp, &secret2);

    node_state.add_account(
        account1.clone(),
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
        account2.clone(),
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

    node_state.get_account(&account1).unwrap();
    node_state.get_account(&account2).unwrap();
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
