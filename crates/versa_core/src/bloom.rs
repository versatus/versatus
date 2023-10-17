use std::{collections::hash_map::DefaultHasher, hash::Hash};

use cuckoofilter::{CuckooError, CuckooFilter};

pub struct Bloom {
    filter: CuckooFilter<DefaultHasher>,
}

impl Clone for Bloom {
    fn clone(&self) -> Self {
        let exported = self.filter.export();
        let filter = CuckooFilter::from(exported);

        Bloom { filter }
    }
}

impl std::fmt::Debug for Bloom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: implement debug for filter
        f.debug_struct("Bloom").finish()
    }
}

impl Bloom {
    pub fn new(limit: usize) -> Bloom {
        Bloom {
            filter: CuckooFilter::with_capacity(limit),
        }
    }

    pub fn contains<T: ?Sized + Hash>(&self, key: &T) -> bool {
        self.filter.contains(&key)
    }

    pub fn push<T: ?Sized + Hash>(&mut self, key: &T) -> Result<(), CuckooError> {
        self.filter.add(&key)
    }

    pub fn delete<T: ?Sized + Hash>(&mut self, key: &T) -> bool {
        self.filter.delete(key)
    }

    pub fn is_empty(&self) -> bool {
        self.filter.is_empty()
    }

    pub fn len(&self) -> usize {
        self.filter.len()
    }

    pub fn memory_usage(&self) -> usize {
        self.filter.memory_usage()
    }
}

#[cfg(test)]
mod tests {

    use super::Bloom;
    #[test]
    fn test_insert_filter() {
        let mut filter = Bloom::new(1000);
        let key = "1";
        filter
            .push(key)
            .expect("test_insert_filter failed to push key");
        assert!(!filter.is_empty());
    }
    #[test]
    fn test_delete_filter() {
        let mut filter = Bloom::new(1000);
        let key = "1";
        filter
            .push(key)
            .expect("test_delete_filter failed to push key");
        assert!(!filter.is_empty());
        filter.delete(key);
        assert!(filter.is_empty());
    }
}
