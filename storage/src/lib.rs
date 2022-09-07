use std::{
    io::{self, BufWriter},
    path::PathBuf,
};

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, StorageError>;

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

pub fn create_vrrb_data_dir() -> PathBuf {
    // creates a vrrb data dir if it doesnt already exist
    // else simply return its location
    todo!();
}

pub fn get_vrrb_data_dir() -> PathBuf {
    // get vrrb data dir from:
    // current working directory
    // environment variables
    // if not present return error
    todo!();
}

pub fn create_node_data_dir() -> io::Result<()> {
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

    pub fn init() {
        //
        // let directory = {
        //     if let Some(dir) = std::env::args().nth(2) {
        //         std::fs::create_dir_all(dir.clone())?;
        //         dir.clone()
        //     } else {
        //         std::fs::create_dir_all("./.vrrb_data".to_string())?;
        //         "./.vrrb_data".to_string()
        //     }
        // };
        //
        // let events_path = format!("{}/events_{}.json", directory.clone(),
        // event_file_suffix); fs::File::create(events_path.clone()).
        // unwrap(); if let Err(err) =
        // write_to_json(events_path.clone(), VrrbNetworkEvent::VrrbStarted) {
        //     info!("Error writting to json in main.rs 164");
        //     telemetry::error!("{:?}", err.to_string());
        // }
        //
        //
    }
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

    #[test]
    fn get_data_dir_returns_right_dir() {
        //
    }
}
