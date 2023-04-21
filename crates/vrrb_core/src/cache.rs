use std::time::Duration;

use lru_time_cache::LruCache;

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

    pub fn get(&mut self, key: &K) -> Option<&V> {
        self.cache.get(key)
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.cache.get_mut(key)
    }

    pub fn push(&mut self, key: K, value: V) {
        self.cache.insert(key, value);
    }

    pub fn contains(&self, key: &K) -> bool {
        self.cache.contains_key(key)
    }

    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn remove(&mut self, key: &K) {
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
        cache.push(1, 1);
        assert!(!cache.is_empty());
    }

    #[test]
    fn test_entry_exist() {
        let mut cache = Cache::new(10, 1000);
        cache.push(1, 1);
        assert!(cache.contains(&1));
    }

    #[test]
    fn test_evict_entries() {
        let mut cache = Cache::new(10, 100);
        cache.push(1, 1);
        assert!(!cache.is_empty());
        sleep(std::time::Duration::from_millis(105));
        assert!(cache.is_empty());
    }

    #[test]
    fn test_clear_cache() {
        let mut cache = Cache::new(10, 100);
        struct Abc {
            _a: i16,
        }
        cache.push("Hello_str", Abc { _a: 1i16 });
        assert!(!cache.is_empty());
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_remove_entry_cache() {
        let mut cache = Cache::new(10, 100);
        struct Abc {
            _a: i16,
        }
        cache.push("Hello_str", Abc { _a: 1i16 });
        assert!(cache.contains(&"Hello_str"));
        cache.remove(&"Hello_str");
        assert!(!cache.contains(&"Hello_str"));
    }
}
