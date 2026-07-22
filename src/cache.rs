use foldhash::{HashMap, HashMapExt};

#[derive(Debug, PartialEq)]
pub enum CacheItem {
    String(String),
    Array(usize, Vec<CacheItem>),
}

impl Clone for CacheItem {
    fn clone(&self) -> Self {
        match self {
            CacheItem::String(s) => CacheItem::String(s.clone()),
            CacheItem::Array(s, items) => CacheItem::Array(*s, items.clone()),
        }
    }
}

pub struct Cache {
    map: HashMap<String, CacheItem>,
}

impl Default for Cache {
    fn default() -> Self {
        Self::new()
    }
}

impl Cache {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn set(&mut self, key: &str, value: CacheItem) {
        self.map.insert(key.to_string(), value);
    }

    pub fn get(&self, key: &str) -> Option<&CacheItem> {
        self.map.get(key)
    }

    pub fn delete(&mut self, key: &str) {
        self.map.remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // sanity checks
    #[test]
    fn test_set_and_get_str() {
        let mut cache = Cache::new();
        cache.set("key", CacheItem::String("value".to_string()));
        let CacheItem::String(str) = cache.get("key").unwrap() else {
            panic!("Expected CacheItem::Value")
        };
        assert_eq!(str, "value");
    }

    #[test]
    fn test_delete_str() {
        let mut cache = Cache::new();
        cache.set("key", CacheItem::String("value".to_string()));
        cache.delete("key");
        let None = cache.get("key") else {
            panic!("Expected Option::None")
        };
    }

    #[test]
    fn test_set_and_get_array() {
        let mut cache = Cache::new();
        let items = vec![
            CacheItem::String("a".to_string()),
            CacheItem::String("b".to_string()),
        ];
        cache.set("key", CacheItem::Array(items.len(), items));
        let CacheItem::Array(len, items) = cache.get("key").unwrap() else {
            panic!("Expected CacheItem::Array")
        };
        assert_eq!(*len, 2);
        assert_eq!(items.len(), 2);
        let CacheItem::String(first) = &items[0] else {
            panic!("Expected CacheItem::String")
        };
        assert_eq!(first, "a");
    }

    #[test]
    fn test_delete_array() {
        let mut cache = Cache::new();
        let items = vec![CacheItem::String("a".to_string())];
        cache.set("key", CacheItem::Array(items.len(), items));
        cache.delete("key");
        let None = cache.get("key") else {
            panic!("Expected Option::None")
        };
    }
}
