use lru_time_cache::LruCache;
use std::time::Duration;

pub struct Cache<K, V> {
    pub limit: usize,
    pub ttl: u64,
    pub cache: LruCache<K, V>,
}

impl<K, V> Cache<K, V>
where
    K: Ord + Clone,
{
    pub fn new(limit: usize, ttl: u64) -> Cache<K, V> {
        Cache {
            limit,
            ttl,
            cache: LruCache::with_expiry_duration_and_capacity(Duration::from_millis(ttl), limit),
        }
    }

    pub fn get_entry(&mut self, key: &K) -> Option<&V> {
        self.cache.get(key)
    }

    pub fn push_to_cache(&mut self, key: K, value: V) {
        self.cache.insert(key, value);
    }

    pub fn check_entry_exist(&self, key: &K) -> bool {
        self.cache.contains_key(key)
    }
    pub fn is_cache_empty(&self) -> bool {
        self.cache.is_empty()
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn remove_from_cache(&mut self,key: &K){
        self.cache.remove(key);
    }
}

#[cfg(test)]
mod tests {

    use std::thread::sleep;

    use super::Cache;
    #[test]
    fn test_insert_cache() {
        let mut cache = Cache::new(10, 1000);
        cache.push_to_cache(1, 1);
        assert!(cache.len() > 0);
    }

    #[test]
    fn test_entry_exist() {
        let mut cache = Cache::new(10, 1000);
        cache.push_to_cache(1, 1);
        assert!(cache.check_entry_exist(&1) == true);
    }

    #[test]
    fn test_evict_entries() {
        let mut cache = Cache::new(10, 100);
        cache.push_to_cache(1, 1);
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
        cache.clear_cache();
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
        cache.remove_from_cache(&"Hello_str"); 
        assert!(!cache.check_entry_exist(&"Hello_str"));
    }
}
