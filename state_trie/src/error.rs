use bincode::ErrorKind;
use lrdb::Account;
use patriecia::error::TrieError;

#[derive(Debug)]
pub enum StateTrieError {
    FailedToGetValueForKey(Vec<u8>, TrieError),
    FailedToDeserializeValue(Vec<u8>),
    FailedToSerializeAccount(Account),
    NoValueForKey,
}
