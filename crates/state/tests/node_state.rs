use std::{env, path::PathBuf};

use state::NodeState;

#[test]
fn can_be_restored_from_file() {
    let temp_dir_path = env::temp_dir();
    let mut db_path = temp_dir_path.clone();
    db_path.join("node.db");

    let network_state = NodeState::new(db_path);
}

#[test]
fn can_be_dumped_into_file() {
    let temp_dir_path = env::temp_dir();
    let mut db_path = temp_dir_path.clone();
    db_path.join("node.db");

    let network_state = NodeState::new(db_path);
}
