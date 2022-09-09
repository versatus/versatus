use std::{io, path::PathBuf};

/// KV-like entity that takes care of managing vrrb's filesystem io
/// creates folders and manages the file size of the files created inside
/// as well as serialization and deserialization of FS data
pub trait Storage {
    fn get();
    fn put();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_data_dir_returns_right_dir() {
        //
    }
}
