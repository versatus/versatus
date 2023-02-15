use patriecia::db::Database;
use primitives::DEFAULT_VRRB_DATA_DIR_PATH;
use rocksdb::{DB, DEFAULT_COLUMN_FAMILY_NAME};

use crate::StateError;

#[derive(Debug)]
pub struct StateBackingDatabase {
    db: DB,
    column: String,
}

fn new_db_instance(
    options: rocksdb::Options,
    path: std::path::PathBuf,
    column_family: &str,
) -> crate::Result<DB> {
    let cfs = rocksdb::DB::list_cf(&options, &path).unwrap_or(vec![]);
    let column_family_exists = cfs.iter().find(|cf| cf == &column_family).is_some();

    let mut instance = rocksdb::DB::open_cf(&options, &path, cfs)
        .map_err(|err| StateError::Other(err.to_string()))?;

    if !column_family_exists {
        let options = rocksdb::Options::default();
        instance
            .create_cf(column_family, &options)
            .map_err(|err| StateError::Other(err.to_string()))?;
    }

    Ok(instance)
}

impl StateBackingDatabase {
    pub fn new(path: std::path::PathBuf, column_family: &str) -> crate::Result<Self> {
        let mut options = rocksdb::Options::default();
        options.set_error_if_exists(false);
        options.create_if_missing(true);
        options.create_missing_column_families(true);

        let instance = new_db_instance(options, path, column_family)
            .map_err(|err| StateError::Other(err.to_string()))?;

        Ok(Self {
            db: instance,
            column: column_family.to_string(),
        })
    }
}

// TODO: handle these unwrap
impl Clone for StateBackingDatabase {
    fn clone(&self) -> Self {
        let mut options = rocksdb::Options::default();
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

impl Default for StateBackingDatabase {
    fn default() -> Self {
        let mut options = rocksdb::Options::default();
        options.set_error_if_exists(false);
        options.create_if_missing(true);
        options.create_missing_column_families(true);

        let db = new_db_instance(
            options,
            DEFAULT_VRRB_DATA_DIR_PATH.into(),
            DEFAULT_COLUMN_FAMILY_NAME,
        )
        .unwrap();

        Self {
            db,
            column: DEFAULT_COLUMN_FAMILY_NAME.to_string(),
        }
    }
}

impl Database for StateBackingDatabase {
    type Error = StateError;

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
