use foldhash::{HashMap, HashMapExt};
use std::time::{Duration, SystemTime};

pub struct ExpirationTable {
    pub map: HashMap<String, SystemTime>,
}

impl Default for ExpirationTable {
    fn default() -> Self {
        Self::new()
    }
}

impl ExpirationTable {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn get(&mut self, key: String) -> Option<SystemTime> {
        self.map.get(&key).cloned()
    }

    pub fn is_expired(&self, key: &str, timestamp: SystemTime) -> bool {
        let expiration_time = self.map.get(key);
        if let Some(expiration_time) = expiration_time {
            return timestamp > *expiration_time;
        };
        false
    }

    /// Adds a key to the expiry table with the expiration timestamp set to now plus the given ttl
    pub fn add(&mut self, key: String, ttl: Duration) {
        let expiration_time = SystemTime::now() + ttl;
        self.map.insert(key, expiration_time);
    }

    pub fn delete(&mut self, key: &str) {
        self.map.remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expiration_table() {
        let mut expiration_table = ExpirationTable::new();
        let timestamp = SystemTime::now();
        expiration_table.add(String::from("foo"), Duration::from_secs(5));

        assert!(!expiration_table.is_expired("foo", timestamp));

        assert!(expiration_table.is_expired("foo", timestamp + Duration::from_secs(6)));
    }
}
