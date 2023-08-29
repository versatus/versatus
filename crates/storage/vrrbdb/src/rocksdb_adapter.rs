use std::collections::hash_map::IntoIter;
use std::collections::{HashMap, BTreeSet};

use anyhow::Result;
use patriecia::{
    LeafNode, Node, NodeBatch, NodeKey, TreeReader, TreeUpdateBatch, TreeWriter,
    VersionedDatabase, OwnedValue, KeyHash, Preimage, StaleNodeIndex, Vers,
};
use primitives::{get_vrrb_environment, Environment, DEFAULT_VRRB_DB_PATH};
use rocksdb::{DB, DEFAULT_COLUMN_FAMILY_NAME, IteratorMode};
use storage_utils::{get_node_data_dir, StorageError};
use telemetry::error;
use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub struct RocksDbAdapter {
    db: Arc<RwLock<DB>>,
    column: String,
    stale_nodes: BTreeSet<StaleNodeIndex>, 
    value_history: HashMap<KeyHash, Vec<(Vers, Option<OwnedValue>)>>,
    preimages: HashMap<KeyHash, Preimage>,
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
            stale_nodes: BTreeSet::new(),
            value_history: HashMap::new(),
            preimages: HashMap::new(),
        })
    }
    
    pub fn write_tree_update_batch(&mut self, batch: TreeUpdateBatch) -> Result<()> {
        self.write_node_batch(&batch.node_batch)?;
        batch.stale_node_index_batch
            .into_iter()
            .map(|i| self.put_stale_node_index(i))
            .collect::<Result<Vec<_>>>()?;
        Ok(())
    }

    pub fn put_stale_node_index(&mut self, index: StaleNodeIndex) -> Result<()> {
        let is_new_entry = self.stale_nodes.insert(index);
        anyhow::ensure!(is_new_entry, "Duplicated retire log");
        Ok(())
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
            stale_nodes: self.stale_nodes.clone(),
            value_history: self.value_history.clone(),
            preimages: self.preimages.clone()
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
            stale_nodes: BTreeSet::new(),
            value_history: HashMap::new(),
            preimages: HashMap::new(),
        }
    }
}

impl VersionedDatabase for RocksDbAdapter {
    type Version = Vers;
    type NodeIter = IntoIter<NodeKey, Node>;
    type HistoryIter = IntoIter<patriecia::KeyHash, Vec<(Vers, Option<OwnedValue>)>>;

    fn get(
        &self,
        max_version: Self::Version,
        node_key: KeyHash,
    ) -> Result<Option<OwnedValue>> {
        self.get_value_option(max_version, node_key)
    }

    fn update_batch(&self, tree_update_batch: TreeUpdateBatch) -> Result<()> {
        self.write_tree_update_batch(tree_update_batch)
    }

    fn nodes(&self) -> IntoIter<NodeKey, Node> {
        let iter = self.db.read().iterator(IteratorMode::Start);
        let mut map = HashMap::new();
        for res in iter {
            match res {
                Ok((boxed_key, boxed_node)) => {
                    let key_bytes = boxed_key.into_vec();
                    let node_bytes = boxed_node.into_vec();
                    if let Ok(node_key) = bincode::deserialize::<NodeKey>(&key_bytes) {
                        if let Ok(node) = bincode::deserialize::<Node>(&node_bytes) {
                            map.insert(node_key, node);
                        }
                    };
                }
                _ => {}
            }
        }

        map.into_iter()
    }

    fn value_history(
        &self,
    ) -> std::collections::hash_map::IntoIter<
        patriecia::KeyHash,
        Vec<(Self::Version, Option<patriecia::OwnedValue>)>
    > {
        self.value_history.clone().into_iter()
    }
}
impl TreeReader for RocksDbAdapter {
    type Version = Vers;

    fn get_node_option(&self, node_key: &NodeKey) -> Result<Option<Node>> {
        let key_bytes = bincode::serialize(node_key)?;
        if let Ok(Some(bytes)) = self.db.get(&key_bytes) {
            if let Ok(node) = bincode::deserialize(&bytes) {
                Ok(Some(node))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    fn get_value_option(
        &self,
        max_version: Vers,
        key_hash: patriecia::KeyHash,
    ) -> Result<Option<patriecia::OwnedValue>> {
        match self.value_history.get(&key_hash) {
            Some(version_history) => {
               for (version, value) in version_history.iter().rev() {
                   if *version <= max_version {
                       return Ok(value.clone())
                   }
               }
               Ok(None)
            },
            None => Ok(None)
        }
    }

    fn get_rightmost_leaf(&self) -> Result<Option<(NodeKey, LeafNode)>> {
        let mut key_and_node: Option<(NodeKey, LeafNode)> = None; 

        let iter = self.db.read().iterator(IteratorMode::Start);
        for res in iter {
            if let Ok((boxed_key, boxed_value)) = res {
                let node_key: NodeKey = bincode::deserialize(&boxed_key.into_vec())?;
                let node_value: Node = bincode::deserialize(&boxed_value.into_vec())?;
                if let Node::Leaf(leaf_node) = node_value {
                    if key_and_node.is_none() {
                        key_and_node.replace((node_key.clone(), leaf_node.clone()));
                    } else if leaf_node.key_hash() > key_and_node.as_ref().unwrap().1.key_hash() {
                        key_and_node.replace((node_key.clone(), leaf_node.clone()));
                    }
                }
            }
        }
        Ok(key_and_node)
    }
}

impl TreeWriter for RocksDbAdapter {
    fn write_node_batch(&self, node_batch: &NodeBatch) -> Result<()> { 
        let mut locked = self.db.write();
        for (node_key, node) in nodes_batch.nodes() {
            let node_key_bytes = bincode::serialize(&node_key)?;
            let node_bytes = bincode::serialize(&node)?;
            self.db.put(node_key_bytes, node_bytes)?;
        }

        for ((version, key_hash), value) in node_batch.values() {
            put_value(
                &mut self.value_history,
                version.into(),
                key_hash,
                value.clone()
            );
        }
        Ok(())
    }
}

pub(crate) fn serialize_version(version: Vers) -> Vec<u8> {
    version.0.to_be_bytes().into()
}

pub(crate) fn deserialize_version(version: Vec<u8>) -> Vers {
    Vers(u64::from_be_bytes(
        version
            .try_into().unwrap_or_default(),
    ))
}

pub fn put_value(
    value_history: &mut HashMap<KeyHash, Vec<(Vers, Option<OwnedValue>)>>,
    version: Vers,
    key: KeyHash,
    value: Option<OwnedValue>,
) -> Result<()> {
    match value_history.entry(key) {
        Entry::Occupied(mut occupied) => {
            if let Some((last_version, last_value)) = occupied.get_mut().last_mut() {
                match version.cmp(last_version) {
                    core::cmp::Ordering::Less => bail!("values must be pushed in order"),
                    core::cmp::Ordering::Equal => {
                        *last_value = value;
                        return Ok(());
                    }
                    // If the new value has a higher version than the previous one, fall through and push it to the array
                    core::cmp::Ordering::Greater => {}
                }
            }
            occupied.get_mut().push((version, value));
        }
        Entry::Vacant(vacant) => {
            vacant.insert(vec![(version, value)]);
        }
    }
    Ok(())
}

