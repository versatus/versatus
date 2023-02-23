use patriecia::db::Database;
use primitives::{base, get_vrrb_environment, Environment, DEFAULT_VRRB_DB_PATH};
use rocksdb::{DB, DEFAULT_COLUMN_FAMILY_NAME};
use storage_utils::{get_node_data_dir, StorageError};

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

    let node_data_dir = get_node_data_dir().unwrap_or_default();

    let log_path = node_data_dir.join("db").join("log");

    options.set_db_log_dir(log_path);

    options
}

fn new_db_instance(
    options: rocksdb::Options,
    path: std::path::PathBuf,
    column_family: &str,
) -> storage_utils::Result<DB> {
    let cfs = rocksdb::DB::list_cf(&options, &path).unwrap_or(vec![]);
    let column_family_exists = cfs.iter().any(|cf| &cf == &column_family);

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

        let db = new_db_instance(
            //
            options,
            self.db.path().into(),
            self.column.as_str(),
        )
        .unwrap();

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
}
