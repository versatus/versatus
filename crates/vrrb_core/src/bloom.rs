use std::collections::hash_map::DefaultHasher;
use cuckoofilter::CuckooFilter;

pub struct Bloom<DefaultHasher> {
   filter:CuckooFilter<DefaultHasher>
}

impl<K> Bloom<K>
where K:DefaultHasher
{
    pub fn new(limit: usize) -> Bloom<K> {
        Bloom {
            filter: CuckooFilter::with_capacity(limit),
        }
    }

    pub fn contains(&self, key:K) -> bool {
        self.filter.contains(&key)
    }

    pub fn push(&mut self, key: K,)->Result<(), cuckoofilter::CuckooError>{
       self.filter.add(&key)
    }

    pub fn remove(&mut self, key: &K) -> bool {
        self.filter.delete(key)
    }
    pub fn is_empty(&self) -> bool {
        self.filter.is_empty()
    }

    pub fn len(&mut self)->usize{
        self.filter.len()
    }

    pub fn memory_usage(&self) -> usize {
        self.filter.memory_usage()
    }
}

#[cfg(test)]
mod tests {

    use std::thread::sleep;

    use super::Bloom;
    #[test]
    fn test_insert_filter() {
        let mut filter = Bloom::new(1000);
        let key="1";
        filter.push(key);
        assert!(filter.len() > 0);
    }

    #[test]
    fn test_entry_exist() {
        let mut cache = Cache::new(10, 1000);
        cache.push(1, 1);
        assert!(cache.check_entry_exist(&1) == true);
    }

    #[test]
    fn test_evict_entries() {
        let mut cache = Cache::new(10, 100);
        cache.push(1, 1);
        assert!(cache.len() > 0);
        sleep(std::time::Duration::from_millis(105));
        assert!(cache.len() == 0);
    }

    #[test]
    fn test_clear_cache() {
        let mut cache = Cache::new(10, 100);
        struct Abc {
            _a: i16,
        }
        cache.push_to_cache("Hello_str", Abc { _a: 1i16 });
        assert!(cache.len() > 0);
        cache.clear();
        assert!(cache.len() == 0);
    }


    #[test]
    fn test_remove_entry_cache() {
        let mut cache = Cache::new(10, 100);
        struct Abc {
            _a: i16,
        }
        cache.push_to_cache("Hello_str", Abc { _a: 1i16 });
        assert!(cache.check_entry_exist(&"Hello_str"));
        cache.remove(&"Hello_str"); 
        assert!(!cache.check_entry_exist(&"Hello_str"));
    }
}

