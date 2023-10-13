use std::path::PathBuf;

use block::{convergence_block, Block, ConvergenceBlock, ProposalBlock};
use ethereum_types::U256;
use patriecia::RootHash;
use primitives::Address;
use storage_utils::{Result, StorageError};
use vrrb_core::transactions::{Transaction, TransactionDigest, TransactionKind, Transfer};
use vrrb_core::{
    account::{Account, UpdateArgs},
    claim::Claim,
};

use crate::{
    ClaimStore, ClaimStoreReadHandleFactory, FromTxn, IntoUpdates, StateStore,
    StateStoreReadHandleFactory, TransactionStore, TransactionStoreReadHandleFactory,
    VrrbDbReadHandle,
};

#[derive(Debug, Clone)]
pub struct VrrbDbConfig {
    pub path: PathBuf,
    pub state_store_path: Option<String>,
    pub transaction_store_path: Option<String>,
    pub event_store_path: Option<String>,
    pub claim_store_path: Option<String>,
}

impl VrrbDbConfig {
    pub fn with_path(&mut self, path: PathBuf) -> Self {
        self.path = path;

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct ApplyBlockResult {
    state_root_hash: RootHash,
    transactions_root_hash: RootHash,
    // claims_root_hash: RootHash,
}

impl ApplyBlockResult {
    pub fn state_root_hash_str(&self) -> String {
        let state_root_hash = self.state_root_hash.clone();
        // let transaction_root_hash = self.transaction_store.root_hash()?;

        // let txn_root_hash_hex = hex::encode(txn_root_hash.0);
        let state_root_hash_hex = hex::encode(state_root_hash.0);
        // let claim_root_hash_hex = hex::encode(claim_root_hash.0);
        state_root_hash_hex
    }

    pub fn transactions_root_hash_str(&self) -> String {
        let txn_root_hash = self.transactions_root_hash.clone();

        let txn_root_hash_hex = hex::encode(txn_root_hash.0);
        txn_root_hash_hex
    }
}

impl Default for VrrbDbConfig {
    fn default() -> Self {
        let path = storage_utils::get_node_data_dir()
            .unwrap_or_default()
            .join("db");

        Self {
            path,
            state_store_path: None,
            transaction_store_path: None,
            event_store_path: None,
            claim_store_path: None,
        }
    }
}

#[derive(Debug, Default)]
pub struct VrrbDb {
    state_store: StateStore,
    transaction_store: TransactionStore,
    claim_store: ClaimStore,
}

impl VrrbDb {
    pub fn new(config: VrrbDbConfig) -> Self {
        let state_store = StateStore::new(&config.path);
        let transaction_store = TransactionStore::new(&config.path);
        let claim_store = ClaimStore::new(&config.path);

        Self {
            state_store,
            transaction_store,
            claim_store,
        }
    }

    pub fn export_state(&self) {
        todo!("implement once integral-db is ready to be consumed");
    }

    pub fn commit_transactions(&mut self) {
        self.transaction_store.commit();
    }

    pub fn commit_state(&mut self) {
        self.state_store.commit();
    }

    pub fn commit_claims(&mut self) {
        self.claim_store.commit();
    }

    pub fn read_handle(&self) -> VrrbDbReadHandle {
        VrrbDbReadHandle::new(
            self.state_store.factory(),
            self.transaction_store_factory(),
            self.claim_store_factory(),
        )
    }

    pub fn new_with_stores(
        state_store: StateStore,
        transaction_store: TransactionStore,
        claim_store: ClaimStore,
    ) -> Self {
        Self {
            state_store,
            transaction_store,
            claim_store,
        }
    }

    /// Returns the current state store trie's root hash.
    pub fn state_root_hash(&self) -> Result<RootHash> {
        self.state_store.root_hash()
    }

    /// Returns the transaction store trie's root hash.
    pub fn transactions_root_hash(&self) -> Result<RootHash> {
        self.transaction_store.root_hash()
    }

    /// Returns the claim store trie's root hash.
    pub fn claims_root_hash(&self) -> Result<RootHash> {
        self.claim_store.root_hash()
    }

    /// Produces a reader factory that can be used to generate read handles into
    /// the state trie.
    pub fn state_store_factory(&self) -> StateStoreReadHandleFactory {
        self.state_store.factory()
    }

    /// Produces a reader factory that can be used to generate read handles into
    /// the the transaction trie.
    pub fn transaction_store_factory(&self) -> TransactionStoreReadHandleFactory {
        self.transaction_store.factory()
    }

    /// Produces a reader factory that can be used to generate read_handles into
    /// the claim trie
    pub fn claim_store_factory(&self) -> ClaimStoreReadHandleFactory {
        self.claim_store.factory()
    }

    /// Inserts an account to current state tree.
    pub fn insert_account(&mut self, key: Address, account: Account) -> Result<()> {
        self.state_store.insert(key, account)
    }

    /// Adds multiplpe accounts to current state tree.
    pub fn extend_accounts(&mut self, accounts: Vec<(Address, Option<Account>)>) {
        self.state_store.extend(accounts);
    }

    /// Updates an account on the current state tree.
    pub fn update_account(&mut self, args: UpdateArgs) -> Result<()> {
        self.state_store
            .update(args)
            .map_err(|err| StorageError::Other(err.to_string()))
    }

    /// Inserts a confirmed transaction to the ledger. Does not check if
    /// accounts involved in the transaction actually exist.
    pub fn insert_transaction_unchecked(&mut self, txn: TransactionKind) -> Result<()> {
        self.transaction_store.insert(txn)
    }

    /// Adds multiplpe transactions to current state tree. Does not check if
    /// accounts involved in the transaction actually exist.
    pub fn extend_transactions_unchecked(&mut self, transactions: Vec<TransactionKind>) {
        self.transaction_store.extend(transactions);
    }

    /// Inserts a confirmed transaction to the ledger. Does not check if
    /// accounts involved in the transaction actually exist.
    pub fn insert_transaction(&mut self, txn: TransactionKind) -> Result<()> {
        self.transaction_store.insert(txn)
    }

    /// Adds multiplpe transactions to current transaction tree. Does not check
    /// if accounts involved in the transaction actually exist.
    pub fn extend_transactions(&mut self, transactions: Vec<TransactionKind>) {
        self.transaction_store.extend(transactions);
    }

    /// Inserts a confirmed claim to the current claim tree.
    pub fn insert_claim_unchecked(&mut self, claim: Claim) -> Result<()> {
        self.claim_store.insert(claim)
    }

    /// Adds multiple claims to the current claim tree.  
    pub fn extend_claims_unchecked(&mut self, claims: Vec<(U256, Option<Claim>)>) {
        self.claim_store.extend(claims)
    }

    /// Inserts a confirmed claim into the claim tree.
    pub fn insert_claim(&mut self, claim: Claim) -> Result<()> {
        self.claim_store.insert(claim)
    }

    /// Inserts multiple claims into the current claim trie
    pub fn extend_claims(&mut self, claims: Vec<(U256, Option<Claim>)>) {
        self.claim_store.extend(claims)
    }

    /// Updates a calim in the current claim trie.
    pub fn update_claim(&mut self, _key: Address, _args: UpdateArgs) {
        todo!()
    }

    fn apply_transfer(&mut self, read_handle: VrrbDbReadHandle, txn: Transfer) -> Result<()> {
        let txn = TransactionKind::Transfer(txn);

        let sender_address = txn.sender_address();
        let receiver_address = txn.receiver_address();

        // TODO: create methods to check if these exist
        read_handle.get_account_by_address(&sender_address)?;
        if let Err(StorageError::Other(_err)) =
            read_handle.get_account_by_address(&receiver_address)
        {
            let account = Account::new(receiver_address.clone());
            self.insert_account(receiver_address.clone(), account)?;
        };

        let updates = IntoUpdates::from_txn(txn.clone());

        self.state_store
            .update_uncommited(sender_address.clone(), updates.sender_update.into())?;

        self.state_store
            .update_uncommited(receiver_address.clone(), updates.receiver_update.into())?;

        self.state_store.commit();

        // TODO: update transaction's state
        self.transaction_store.insert(txn)?;

        Ok(())
    }

    fn apply_txn(
        &mut self,
        read_handle: VrrbDbReadHandle,
        txn_kind: TransactionKind,
    ) -> Result<()> {
        match txn_kind {
            TransactionKind::Transfer(txn) => self.apply_transfer(read_handle, txn),
            _ => {
                telemetry::info!("unsupported transaction type: {:?}", txn_kind);
                Err(StorageError::Other(
                    "unsupported transaction type".to_string(),
                ))
            },
        }
    }

    pub fn apply_convergence_block(
        &mut self,
        convergence: &ConvergenceBlock,
        proposals: &[ProposalBlock],
    ) -> Result<ApplyBlockResult> {
        let read_handle = self.read_handle();
        for (proposal, txn_set) in &convergence.txns {
            let block = proposals
                .iter()
                .find(|pblock| pblock.hash == proposal.clone())
                .ok_or(StorageError::Other(format!(
                    "unable to find proposal block with hash {}",
                    &proposal
                )))?;

            let mut txns = block.txns.clone();
            txns.retain(|digest, _| txn_set.contains(digest));
            for (digest, txn_kind) in txns {
                self.apply_txn(read_handle.clone(), txn_kind)?;
            }
        }

        self.transaction_store.commit();
        self.state_store.commit();

        let state_root_hash = self.state_store.root_hash()?;
        let transactions_root_hash = self.transaction_store.root_hash()?;

        Ok(ApplyBlockResult {
            state_root_hash,
            transactions_root_hash,
        })
    }

    /// Applies a block of transactions updating the account states accordingly.
    pub fn apply_block(&mut self, block: Block) -> Result<ApplyBlockResult> {
        let read_handle = self.read_handle();

        // TODO: check transactions length and return error if empty

        match block {
            Block::Genesis { block } => {
                if block.txns.is_empty() {
                    return Err(StorageError::Other(
                        "genesis block must contain at least one transaction".to_string(),
                    ));
                }
                for (_, txn_kind) in block.txns {
                    self.apply_txn(read_handle.clone(), txn_kind)?;
                }
            },
            Block::Convergence { .. } => {
                todo!()
            },
            _ => {
                telemetry::info!("unsupported block type: {:?}", block);
                return Err(StorageError::Other("unsupported block type".to_string()));
            },
        }

        self.transaction_store.commit();
        self.state_store.commit();

        let state_root_hash = self.state_store.root_hash()?;
        let transactions_root_hash = self.transaction_store.root_hash()?;
        // let claim_root_hash = self.claim_store.root_hash()?;
        // let claim_root_hash_hex = hex::encode(claim_root_hash.0);

        Ok(ApplyBlockResult {
            state_root_hash,
            transactions_root_hash,
        })
    }
}

impl Clone for VrrbDb {
    fn clone(&self) -> VrrbDb {
        Self {
            state_store: self.state_store.clone(),
            transaction_store: self.transaction_store.clone(),
            claim_store: self.claim_store.clone(),
        }
    }
}

// TODO: uncomment this once `entries` is fixed
// impl Display for VrrbDb {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         let state_entries = self.state_store_factory().handle().entries();
//         let transaction_entries = self
//             .transaction_store_factory()
//             .handle()
//             .entries()
//             .into_iter()
//             .map(|(digest, txn)| (digest.to_string(), txn))
//             .collect::<HashMap<String, Txn>>();
//         let claim_entries = self.claim_store_factory().handle().entries();

//         let out = json!({
//             "state": {
//                 "count": state_entries.len(),
//                 "entries": state_entries,
//             },
//             "transactions": {
//                 "count": transaction_entries.len(),
//                 "entries": transaction_entries,
//             },
//             "claims": {
//                 "count": claim_entries.len(),
//                 "entries": claim_entries,
//             },
//         });

//         //TODO: report errors better
//         let out_str = serde_json::to_string_pretty(&out).map_err(|_| std::fmt::Error)?;

//         f.write_str(&out_str)
//     }
// }
