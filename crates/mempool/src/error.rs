use primitives::TxHashString;

#[derive(thiserror::Error, PartialEq, Eq, Debug)]
pub enum MempoolError {
    #[error("transaction {0} was not found in mempool")]
    TransactionMissing(TxHashString),

    #[error("invalid transaction {0}")]
    TransactionInvalid(TxHashString),

    #[error("transaction {0} already exists")]
    TransactionExists(TxHashString),
}
