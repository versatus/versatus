use left_right::Absorb;
pub use left_right::ReadHandleFactory;
use patriecia::{db::Database, inner::InnerTrie, trie::Trie};

use crate::Operation;

impl<D> Absorb<Operation> for InnerTrie<D>
where
    D: Database,
{
    fn absorb_first(&mut self, operation: &mut Operation, _other: &Self) {
        match operation {
            // TODO: report errors via instrumentation
            Operation::Add(key, value) => {
                self.insert(key, value).unwrap_or_default();
                self.commit().unwrap_or_default();
                dbg!(self.len());
                dbg!(self.db().len());
            },
            Operation::Update(key, value) => {
                self.insert(key, value).unwrap_or_default();
                self.commit().unwrap_or_default();
            },
            Operation::Remove(key) => {
                self.remove(key).unwrap_or_default();
            },
        }
    }

    fn sync_with(&mut self, first: &Self) {
        *self = first.clone();
    }
}
