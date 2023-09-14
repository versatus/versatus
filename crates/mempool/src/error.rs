use vrrb_core::transactions::TransactionDigest;

#[derive(thiserror::Error, PartialEq, Eq, Debug)]
pub enum MempoolError {
    #[error("transaction {0} was not found in mempool")]
    #[deprecated]
    TransactionMissing(TransactionDigest),

    #[error("transaction {0} was not found in mempool")]
    TransactionNotFound(TransactionDigest),

    #[error("invalid transaction {0}")]
    TransactionInvalid(TransactionDigest),

    #[error("transaction {0} already exists")]
    TransactionExists(TransactionDigest),
}
