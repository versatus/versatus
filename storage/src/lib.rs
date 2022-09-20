use std::{
    env, fs,
    io::{self, BufWriter},
    os,
    path::PathBuf,
};

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

/// Creates a data dir if it doesn't exists already, otherwise it simply returns its path
pub fn create_vrrb_data_dir() -> Result<PathBuf> {
    let path = get_vrrb_data_dir()?;

    fs::create_dir_all(&path)?;

    Ok(path)
}

/// Gets the data directory path from environment variables or the default location.
pub fn get_vrrb_data_dir() -> Result<PathBuf> {
    let vrrb_data_dir =
        env::var("VRRB_DATA_DIR_PATH").unwrap_or_else(|_| DEFAULT_VRRB_DATA_DIR_PATH.into());

    Ok(vrrb_data_dir.into())
}

// Node specific helpers
// ============================================================================

pub fn create_node_data_dir() -> io::Result<()> {
    // see if node data dir exists within vrrb data dir
    // if so, read it and return its path
    // else create it within vrrb's data dir
    // and populate it with the node's config and data outs
    //
    todo!();
}

pub fn get_node_data_dir() -> io::Result<()> {
    // see if node data dir exists within vrrb data dir
    // if so, read it and return its path
    // else create it within vrrb's data dir
    // and populate it with the node's config and data outs
    //
    todo!();
}

pub struct FileSystemStorage {
    // buf: BufWriter<std::fs::File>,
    data_dir: PathBuf,
}

impl FileSystemStorage {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    fn init() {}
}

impl Default for FileSystemStorage {
    fn default() -> Self {
        Self {
            data_dir: String::from(".vrrb").into(),
        }
    }
}

impl Storage for FileSystemStorage {
    fn get() -> Result<Vec<u8>> {
        todo!()
    }

    fn put() {
        todo!()
    }

    fn append(key: String, value: Vec<u8>) -> Result<Vec<u8>> {
        todo!()
    }

    fn remove() {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

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
}
