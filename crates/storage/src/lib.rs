use std::{env, fs, path::PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, StorageError>;

const DEFAULT_VRRB_DATA_DIR_PATH: &str = ".vrrb";

/// KV-like entity that takes care of managing vrrb's filesystem I/O
/// creates folders and manages the file size of the files created inside
/// as well as serialization and deserialization of FS data
pub trait Storage {
    fn get() -> Result<Vec<u8>>;
    fn put();
    /// Accepts a key, which represents a file namespace and a value to append
    /// to that file namespace
    fn append(key: String, value: Vec<u8>) -> Result<Vec<u8>>;
    fn remove();
}

/// Creates a data dir if it doesn't exists already, otherwise it simply returns
/// its path
pub fn create_vrrb_data_dir() -> Result<PathBuf> {
    let path = get_vrrb_data_dir()?;

    fs::create_dir_all(&path)?;

    Ok(path)
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

pub struct FileSystemStorageDriver {
    // buf: BufWriter<std::fs::File>,
    _data_dir: PathBuf,
}

impl FileSystemStorageDriver {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            _data_dir: data_dir,
        }
    }

    fn _init() {}
}

impl Default for FileSystemStorageDriver {
    fn default() -> Self {
        Self {
            _data_dir: String::from(".vrrb").into(),
        }
    }
}

impl Storage for FileSystemStorageDriver {
    fn get() -> Result<Vec<u8>> {
        todo!()
    }

    fn put() {
        todo!()
    }

    fn append(_key: String, _value: Vec<u8>) -> Result<Vec<u8>> {
        todo!()
    }

    fn remove() {
        todo!()
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
