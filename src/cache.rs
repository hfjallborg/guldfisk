use foldhash::{HashMap, HashMapExt};

pub struct Cache {
    map: HashMap<String, Vec<u8>>,
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

    pub fn set(&mut self, key: &str, value: Vec<u8>) {
        self.map.insert(key.to_string(), value);
    }

    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.map.get(key).cloned()
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
    fn test_set_and_get() {
        let mut cache = Cache::new();
        cache.set("key", b"value".to_vec());
        assert_eq!(cache.get("key"), Some(b"value".to_vec()));
    }

    #[test]
    fn test_delete() {
        let mut cache = Cache::new();
        cache.set("key", b"value".to_vec());
        cache.delete("key");
        assert_eq!(cache.get("key"), None);
    }
}
