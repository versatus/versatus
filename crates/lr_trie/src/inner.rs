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
            },
            Operation::Remove(key) => {
                self.remove(key).unwrap_or_default();
            },
            Operation::Extend(values) => {
                //
                // TODO: temp hack to get this going. Refactor ASAP
                //
                for (k, v) in values {
                    self.insert(k, v).unwrap_or_default();
                }
                self.commit().unwrap_or_default();
            },
        }
    }

    fn sync_with(&mut self, first: &Self) {
        *self = first.clone();
    }
}
