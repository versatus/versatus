
#[derive(PartialEq, Eq, Debug)]
pub enum MempoolError {
    TransactionMissing,
    TransactionInvalid,
    TransactionExists
}
