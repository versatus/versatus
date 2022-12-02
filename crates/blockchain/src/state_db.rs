// TODO: cleanup
pub struct ChainDb {}

// TODO: cleanup
impl ChainDb {
    /// Loads the chain from binary and returns a PickleDB instance
    pub fn get_chain_db(&self) -> PickleDb {
        match PickleDb::load(
            self.chain_db.clone(),
            PickleDbDumpPolicy::DumpUponRequest,
            SerializationMethod::Bin,
        ) {
            Ok(nst) => nst,
            Err(_) => PickleDb::new(
                self.chain_db.clone(),
                PickleDbDumpPolicy::DumpUponRequest,
                SerializationMethod::Bin,
            ),
        }
    }

    /// Creates a clone of a PickleDB Instance containing chain data.
    pub fn clone_chain_db(&self) -> PickleDb {
        let db = self.get_chain_db();
        let keys = db.get_all();

        let mut cloned_db = PickleDb::new(
            format!("temp_{}.db", self.chain_db.clone()),
            PickleDbDumpPolicy::NeverDump,
            SerializationMethod::Bin,
        );

        keys.iter().for_each(|k| {
            let block = db.get::<Block>(k);
            if let Some(block) = block {
                if let Err(e) = cloned_db.set(k, &block) {
                    println!(
                        "Error setting block with last_hash {} to cloned_db: {:?}",
                        k, e
                    );
                }
            }
        });

        drop(db);
        cloned_db
    }

    /// Serializes the Chain Database into a string
    pub fn chain_db_to_string(&self) -> String {
        let db = self.clone_chain_db();
        let mut db_map = LinkedHashMap::new();
        let keys = db.get_all();

        for key in keys.iter() {
            let value = db.get::<Block>(key).unwrap();
            let k = key.clone();
            db_map.insert(k, value);
        }

        serde_json::to_string(&db_map).unwrap()
    }

    /// Serializes the Chain Database into a vector of bytes of any size
    pub fn chain_db_to_bytes(&self) -> Vec<u8> {
        self.chain_db_to_string().as_bytes().to_vec()
    }

    /// Deserializes a slice of bytes into a PickleDB Instance
    pub fn chain_db_from_bytes(&self, data: &[u8]) -> PickleDb {
        let db_map = serde_json::from_slice::<LinkedHashMap<String, Block>>(data).unwrap();

        let mut db = PickleDb::new(
            self.clone().chain_db,
            PickleDbDumpPolicy::DumpUponRequest,
            SerializationMethod::Bin,
        );

        db_map.iter().for_each(|(k, v)| {
            if let Err(e) = db.set(k, &v) {
                println!("Error setting block in database: {:?}", e);
            };
        });

        db
    }

    /// Dumps data to a PickleDB Instance
    pub fn dump(&self, block: &Block) -> Result<(), Box<dyn Error>> {
        let mut db = self.get_chain_db();
        if let Err(e) = db.set(&block.header.last_hash, block) {
            return Err(Box::new(e));
        }

        if let Err(e) = db.dump() {
            return Err(Box::new(e));
        }

        Ok(())
    }

    /// Retrieves a block based on the `last_hash` field. Returns an option
    /// (Some(Block) if the block exists in the db, None if it does not)
    pub fn get_block(&self, last_hash: &str) -> Option<Block> {
        let db = self.get_chain_db();
        db.get::<Block>(last_hash)
    }

    /// Serializes a chain into bytes
    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_string().as_bytes().to_vec()
    }

    /// Deserialize a slice of bytes into a blockchain
    pub fn from_bytes(data: &[u8]) -> Result<Blockchain, serde_json::Error> {
        match serde_json::from_slice::<Blockchain>(data) {
            Ok(chain) => Ok(chain),
            Err(e) => Err(e),
        }
    }

    /// Serializes a chain into a string
    pub fn serialize_to_string(&self) -> Result<String, serde_json::Error> {
        match serde_json::to_string(self) {
            Ok(chain_str) => Ok(chain_str),
            Err(e) => Err(e),
        }
    }

    /// Deserializes a string slice into a chain
    pub fn from_string(data: &str) -> Blockchain {
        serde_json::from_str(data).unwrap()
    }

    /// Returns a vector of all the field names of a chain.
    pub fn get_field_names(&self) -> Vec<String> {
        vec![
            "genesis".to_string(),
            "child".to_string(),
            "parent".to_string(),
            "chain".to_string(),
            "chain_db".to_string(),
            "block_cache".to_string(),
            "future_blocks".to_string(),
            "invalid".to_string(),
            "updating_state".to_string(),
            "state_update_cache".to_string(),
        ]
    }
}

impl GettableFields for ChainDb {
    fn get_field(&self, field: &str) -> Option<String> {
        match field {
            "genesis" => self.genesis.clone().map(|g| g.to_string()),
            "child" => self.child.clone().map(|c| c.to_string()),
            "parent" => self.parent.clone().map(|p| p.to_string()),
            "chain" => Some(serde_json::to_string(&self.chain).unwrap()),
            "chain_db" => Some(self.chain_db.clone()),
            "block_cache" => Some(serde_json::to_string(&self.block_cache).unwrap()),
            "future_blocks" => Some(serde_json::to_string(&self.future_blocks).unwrap()),
            "invalid" => Some(serde_json::to_string(&self.invalid).unwrap()),
            "updating_state" => Some(format!("{}", self.updating_state)),
            "state_update_cache" => Some(serde_json::to_string(&self.state_update_cache).unwrap()),
            _ => None,
        }
    }
}
