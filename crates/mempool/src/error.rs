use primitives::TxHashString;

#[derive(thiserror::Error, PartialEq, Eq, Debug)]
pub enum MempoolError {
    #[error("transaction {0} was not found in mempool")]
    #[deprecated]
    TransactionMissing(TxHashString),

    #[error("transaction {0} was not found in mempool")]
    TransactionNotFound(TxHashString),

    #[error("invalid transaction {0}")]
    TransactionInvalid(TxHashString),

    #[error("transaction {0} already exists")]
    TransactionExists(TxHashString),
}
