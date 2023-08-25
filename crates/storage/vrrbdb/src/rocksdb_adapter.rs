use anyhow::Result;
use patriecia::{
    db::Database, LeafNode, Node, NodeBatch, NodeKey, TreeReader, TreeUpdateBatch, TreeWriter,
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

impl Database for RocksDbAdapter {
    type Error = StorageError;

    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Self::Error> {
        self.db
            .get(key)
            .map_err(|err| Self::Error::Other(err.to_string()))
    }

    fn insert(&self, key: &[u8], value: Vec<u8>) -> Result<(), Self::Error> {
        self.db
            .put(key, value)
            .map_err(|err| Self::Error::Other(err.to_string()))
    }

    fn remove(&self, key: &[u8]) -> Result<(), Self::Error> {
        self.db
            .delete(key)
            .map_err(|err| Self::Error::Other(err.to_string()))
    }

    fn flush(&self) -> Result<(), Self::Error> {
        self.db
            .flush()
            .map_err(|err| Self::Error::Other(err.to_string()))
    }

    fn len(&self) -> Result<usize, Self::Error> {
        Ok(self.db.iterator(rocksdb::IteratorMode::Start).count())
    }

    fn is_empty(&self) -> Result<bool, Self::Error> {
        let count = self.len().unwrap_or(0);

        Ok(count == 0)
    }

    /// NOTE: broken, do not use yet
    fn values(&self) -> Result<Vec<(Vec<u8>, Vec<u8>)>, Self::Error> {
        let values = self
            .db
            .iterator(rocksdb::IteratorMode::Start)
            .filter_map(|res| match res {
                Ok((k, v)) => Some((k.into(), v.into())),

                _ => None,
            })
            .collect::<Vec<(Vec<u8>, Vec<u8>)>>();

        Ok(values)
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
