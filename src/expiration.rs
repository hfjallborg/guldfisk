use foldhash::{HashMap, HashMapExt};
use rand::seq::IteratorRandom;
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

    pub fn get(&mut self, key: &str) -> Option<SystemTime> {
        self.map.get(key).cloned()
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

    /// Randomly samples 25 % of keys with a TTL and returns the ones that have expired as of the given timestamp
    pub fn sample_expired(&self, timestamp: SystemTime) -> Vec<String> {
        let count = self.map.len() / 4;
        let mut rng = rand::rng();
        self.map
            .iter()
            .sample(&mut rng, count)
            .into_iter()
            .filter(|(key, _)| self.is_expired(key, timestamp))
            .map(|(key, _)| key.clone())
            .collect()
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

    #[test]
    fn test_sample_expired_returns_expired_keys() {
        let mut expiration_table = ExpirationTable::new();
        for i in 0..8 {
            expiration_table.add(format!("key{}", i), Duration::from_secs(60));
        }

        // sample as if it happened after every key's TTL had elapsed
        let past_expiration = SystemTime::now() + Duration::from_secs(61);
        let expired = expiration_table.sample_expired(past_expiration);

        // 25% of 8 entries
        assert_eq!(expired.len(), 2);
        for key in expired {
            assert!(expiration_table.map.contains_key(&key));
        }
    }

    #[test]
    fn test_sample_expired_skips_keys_that_have_not_expired_yet() {
        let mut expiration_table = ExpirationTable::new();
        for i in 0..8 {
            expiration_table.add(format!("key{}", i), Duration::from_secs(60));
        }

        let expired = expiration_table.sample_expired(SystemTime::now());

        assert_eq!(expired.len(), 0);
    }
}
