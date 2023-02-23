// impl From<NodeStateValues> for NodeState {
//     fn from(node_state_values: NodeStateValues) -> Self {
//         let mut state_db = StateDb::new();
//         let mut txn_db = TxnDb::new();
//
//         let mapped_state = node_state_values
//             .state
//             .into_iter()
//             .map(|(key, acc)| (key, acc))
//             .collect();
//
//         state_db.extend(mapped_state);
//
//         Self {
//             path: PathBuf::new(),
//             state_db,
//             txn_db,
//             mempool: todo!(),
//         }
//     }
// }
//
// #[derive(Debug, Default, Serialize, Deserialize)]
// struct NodeStateValues {
//     pub txns: HashMap<TxHashString, Txn>,
//     pub state: HashMap<SerializedPublicKeyString, Account>,
// }
//
// impl From<&NodeState> for NodeStateValues {
//     fn from(node_state: &NodeState) -> Self {
//         Self {
//             txns: HashMap::new(),
//             state: node_state.entries(),
//         }
//     }
// }
//
// impl NodeStateValues {
//     /// Converts a vector of bytes into a Network State or returns an error
// if     /// it's unable to.
//     fn from_bytes(data: ByteSlice) -> Result<NodeStateValues> {
//         serde_helpers::decode_bytes(data).map_err(|err|
// StateError::Other(err.to_string()))     }
// }
//
// impl From<ByteVec> for NodeStateValues {
//     fn from(data: ByteVec) -> Self {
//         Self::from_bytes(&data).unwrap_or_default()
//     }
// }
//
// impl<'a> From<ByteSlice<'a>> for NodeStateValues {
//     fn from(data: ByteSlice) -> Self {
//         Self::from_bytes(data).unwrap_or_default()
//     }
// }
