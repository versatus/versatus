use microkv::MicroKV;
use patriecia::Database;
use storage_utils::StorageError;

#[derive(Clone)]
pub struct MicroKvAdapter {
    db: MicroKV,
}

impl MicroKvAdapter {
    pub fn new(
        path: std::path::PathBuf,
        dbname: &str,
        auto_commit: bool,
    ) -> storage_utils::Result<Self> {
        let mut db = microkv::MicroKV::open_with_base_path(dbname, path)
            .map_err(|err| StorageError::Other(err.to_string()))?;

        if auto_commit {
            db = db.set_auto_commit(true);
        }

        db.commit()
            .map_err(|err| StorageError::Other(err.to_string()))?;

        Ok(Self { db })
    }
}

impl std::fmt::Debug for MicroKvAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MicroKvAdapter")
            // .field("path", &self.path)
            // .field("storage", &self.storage)
            // .field("nonce", &self.nonce)
            // .field("pwd", &self.pwd)
            // .field("is_auto_commit", &self.is_auto_commit)
            .finish()
    }
}

impl Default for MicroKvAdapter {
    fn default() -> Self {
        let db = MicroKV::new("versatus_default");
        Self { db }
    }
}

impl Database for MicroKvAdapter {
    type Error = StorageError;

    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Self::Error> {
        let key = hex::encode(key);
        self.db
            .get(key)
            .map_err(|err| Self::Error::Other(err.to_string()))
    }

    fn insert(&self, key: &[u8], value: Vec<u8>) -> Result<(), Self::Error> {
        let key = hex::encode(key);
        self.db
            .put(key, &value)
            .map_err(|err| Self::Error::Other(err.to_string()))
    }

    fn remove(&self, key: &[u8]) -> Result<(), Self::Error> {
        let key = hex::encode(key);
        self.db
            .delete(key)
            .map_err(|err| Self::Error::Other(err.to_string()))
    }

    fn flush(&self) -> Result<(), Self::Error> {
        self.db
            .commit()
            .map_err(|err| Self::Error::Other(err.to_string()))
    }

    fn len(&self) -> Result<usize, Self::Error> {
        let keys = self
            .db
            .keys()
            .map_err(|err| Self::Error::Other(err.to_string()))?;

        Ok(keys.len())
    }

    fn is_empty(&self) -> Result<bool, Self::Error> {
        let keys = self
            .db
            .keys()
            .map_err(|err| Self::Error::Other(err.to_string()))?;

        Ok(keys.is_empty())
    }

    /// NOTE: broken, do not use yet
    fn values(&self) -> Result<Vec<(Vec<u8>, Vec<u8>)>, Self::Error> {
        // self.db.
        // let values = self
        //     .db
        //     .iterator(rocksdb::IteratorMode::Start)
        //     .filter_map(|res| match res {
        //         Ok((k, v)) => Some((k.into(), v.into())),
        //
        //         _ => None,
        //     })
        //     .collect::<Vec<(Vec<u8>, Vec<u8>)>>();
        //
        // Ok(values)
        todo!()
    }
}
