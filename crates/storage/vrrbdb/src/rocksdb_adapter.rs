use anyhow::Result;
use patriecia::{
    LeafNode, Node, NodeBatch, NodeKey, TreeReader, TreeUpdateBatch, TreeWriter, Version,
    VersionedDatabase,
};
use primitives::{get_vrrb_environment, Environment, DEFAULT_VRRB_DB_PATH};
use rocksdb::{DB, DEFAULT_COLUMN_FAMILY_NAME};
use storage_utils::{get_node_data_dir, StorageError};
use telemetry::error;

#[derive(Debug)]
pub struct RocksDbAdapter {
    db: DB,
    column: String,
}

fn base_db_options() -> rocksdb::Options {
    let mut options = rocksdb::Options::default();

    let environ = get_vrrb_environment();

    if matches!(environ, Environment::Local) {
        options.set_keep_log_file_num(3);
    }

    match get_node_data_dir() {
        Ok(node_data_dir) => {
            let log_path = node_data_dir.join("db").join("log");
            options.set_db_log_dir(log_path);
        },
        Err(err) => {
            error!("could not get node data directory: {}", err);
            let default_data_dir = std::path::PathBuf::default();
            let log_path = default_data_dir.join("db").join("log");
            options.set_db_log_dir(log_path);
        },
    }

    options
}

fn new_db_instance(
    options: rocksdb::Options,
    path: std::path::PathBuf,
    column_family: &str,
) -> storage_utils::Result<DB> {
    let cfs = match rocksdb::DB::list_cf(&options, &path) {
        Ok(cfs) => cfs,
        Err(err) => {
            error!(
                "could not find column families at {}: {}",
                path.display(),
                err.into_string()
            );
            vec![]
        },
    };

    let column_family_exists = cfs.iter().any(|cf| cf == column_family);

    let mut instance = rocksdb::DB::open_cf(&options, &path, cfs)
        .map_err(|err| StorageError::Other(err.to_string()))?;

    if !column_family_exists {
        let options = base_db_options();

        instance
            .create_cf(column_family, &options)
            .map_err(|err| StorageError::Other(err.to_string()))?;
    }

    Ok(instance)
}

impl RocksDbAdapter {
    pub fn new(path: std::path::PathBuf, column_family: &str) -> storage_utils::Result<Self> {
        let mut options = base_db_options();
        options.set_error_if_exists(false);
        options.create_if_missing(true);
        options.create_missing_column_families(true);

        let instance = new_db_instance(options, path, column_family)
            .map_err(|err| StorageError::Other(err.to_string()))?;

        Ok(Self {
            db: instance,
            column: column_family.to_string(),
        })
    }
}

// TODO: handle these unwrap
impl Clone for RocksDbAdapter {
    fn clone(&self) -> Self {
        let mut options = base_db_options();
        options.set_error_if_exists(false);

        let db = new_db_instance(options, self.db.path().into(), self.column.as_str()).unwrap();

        Self {
            db,
            column: self.column.clone(),
        }
    }
}

impl Default for RocksDbAdapter {
    fn default() -> Self {
        let mut options = base_db_options();
        options.set_error_if_exists(false);
        options.create_if_missing(true);
        options.create_missing_column_families(true);

        //
        // TODO: fix this unwrap
        let db = new_db_instance(
            options,
            DEFAULT_VRRB_DB_PATH.into(),
            DEFAULT_COLUMN_FAMILY_NAME,
        )
        .unwrap();

        Self {
            db,
            column: DEFAULT_COLUMN_FAMILY_NAME.to_string(),
        }
    }
}

impl VersionedDatabase for RocksDbAdapter {
    fn get(
        &self,
        max_version: patriecia::Version,
        node_key: patriecia::KeyHash,
    ) -> Result<Option<patriecia::OwnedValue>> {
        todo!()
    }

    fn update_batch(&self, tree_update_batch: TreeUpdateBatch) -> Result<()> {
        todo!()
    }

    fn nodes(&self) -> std::collections::HashMap<NodeKey, Node> {
        todo!()
    }

    fn value_history(
        &self,
    ) -> std::collections::HashMap<
        patriecia::KeyHash,
        Vec<(patriecia::Version, Option<patriecia::OwnedValue>)>,
    > {
        todo!()
    }
}
impl TreeReader for RocksDbAdapter {
    fn get_node_option(&self, node_key: &NodeKey) -> Result<Option<Node>> {
        todo!()
    }

    fn get_value_option(
        &self,
        max_version: patriecia::Version,
        key_hash: patriecia::KeyHash,
    ) -> Result<Option<patriecia::OwnedValue>> {
        todo!()
    }

    fn get_rightmost_leaf(&self) -> Result<Option<(NodeKey, LeafNode)>> {
        todo!()
    }
}
impl TreeWriter for RocksDbAdapter {
    fn write_node_batch(&self, node_batch: &NodeBatch) -> Result<()> {
        todo!()
    }
}

pub(crate) fn serialize_version(version: Version) -> Vec<u8> {
    version.to_be_bytes().into()
}

pub(crate) fn deserialize_version(version: Vec<u8>) -> Version {
    u64::from_be_bytes(
        version
            .try_into()
            .expect("failed to convert `Vec<u8>` into `[u8; 8]`"),
    )
}
