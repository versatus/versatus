use std::{
    env, fs,
    fs::File,
    path::{Path, PathBuf},
};

mod result;

use primitives::DEFAULT_VRRB_DATA_DIR_PATH;

pub use crate::result::{Result, StorageError};

/// Creates a data dir if it doesn't exists already, otherwise it simply returns
/// its path
pub fn create_vrrb_data_dir() -> Result<PathBuf> {
    let path = get_vrrb_data_dir()?;

    fs::create_dir_all(&path)?;

    Ok(path)
}

/// Removes a data dir if it already exists.
pub fn remove_vrrb_data_dir() {
    let path = get_vrrb_data_dir().unwrap();
    if path.exists() {
        std::fs::remove_dir_all(path).expect("failed to remove .vrrb directory");
    }
}

/// Gets the data directory path from environment variables or the default
/// location.
pub fn get_vrrb_data_dir() -> Result<PathBuf> {
    let vrrb_data_dir =
        env::var("VRRB_DATA_DIR_PATH").unwrap_or_else(|_| DEFAULT_VRRB_DATA_DIR_PATH.into());

    Ok(vrrb_data_dir.into())
}

// Node specific helpers
// ============================================================================
/// Initializes the node specific data directory.
pub fn create_node_data_dir() -> Result<PathBuf> {
    let path = get_node_data_dir()?;
    fs::create_dir_all(&path)?;
    Ok(path)
}

/// Retrieves the node's data directory path.
pub fn get_node_data_dir() -> Result<PathBuf> {
    let mut vrrb_data_dir = get_vrrb_data_dir()?;
    vrrb_data_dir.push("node");
    Ok(vrrb_data_dir)
}

pub fn read_file<F: AsRef<Path>>(path: F) -> Result<File> {
    match File::open(path.as_ref()) {
        Ok(file) => Ok(file),
        Err(e) => Err(StorageError::Io(e)),
    }
}

pub fn create_dir<F: AsRef<Path>>(outdir: F) -> Result<()> {
    match fs::create_dir_all(outdir) {
        Ok(_) => Ok(()),
        Err(e) => Err(StorageError::Io(e)),
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;

    #[test]
    #[serial]
    fn get_vrrb_data_dir_returns_correct_directory() {
        env::remove_var("VRRB_DATA_DIR_PATH");
        let dir = get_vrrb_data_dir().unwrap();
        assert_eq!(dir, PathBuf::from(DEFAULT_VRRB_DATA_DIR_PATH));

        let temp_dir_path = env::temp_dir();
        env::set_var("VRRB_DATA_DIR_PATH", &temp_dir_path);

        let dir = get_vrrb_data_dir().unwrap();
        assert_eq!(dir, temp_dir_path);
    }

    #[test]
    #[serial]
    fn create_vrrb_data_dir_creates_dir_in_path() {
        env::remove_var("VRRB_DATA_DIR_PATH");

        let temp_dir_path = env::temp_dir();

        env::set_var("VRRB_DATA_DIR_PATH", &temp_dir_path);

        let dir = create_vrrb_data_dir().unwrap();
        assert_eq!(dir, temp_dir_path);
    }

    #[test]
    #[serial]
    fn get_node_data_dir_returns_correct_directory() {
        env::remove_var("VRRB_DATA_DIR_PATH");
        let dir = get_node_data_dir().unwrap();

        let mut default_vrrb_data_dir = PathBuf::from(DEFAULT_VRRB_DATA_DIR_PATH);
        default_vrrb_data_dir.push("node");
        assert_eq!(dir, default_vrrb_data_dir);

        let mut temp_dir_path = env::temp_dir();
        env::set_var("VRRB_DATA_DIR_PATH", &temp_dir_path);
        temp_dir_path.push("node");

        let dir = get_node_data_dir().unwrap();
        assert_eq!(dir, temp_dir_path);
    }

    #[test]
    #[serial]
    fn create_node_data_dir_creates_dir_in_path() {
        env::remove_var("VRRB_DATA_DIR_PATH");

        let mut temp_dir_path = env::temp_dir();
        env::set_var("VRRB_DATA_DIR_PATH", &temp_dir_path);

        let dir = create_node_data_dir().unwrap();

        // modify the data dir so it matches the default node path
        temp_dir_path.push("node");
        assert_eq!(dir, temp_dir_path);
    }
}
