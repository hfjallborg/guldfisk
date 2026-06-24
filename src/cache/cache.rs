use foldhash::{HashMap, HashMapExt};

pub struct Cache {
    map: HashMap<String, String>,
}

impl Cache {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn set(&mut self, key: &str, value: &str) {
        self.map.insert(key.to_string(), value.to_string());
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.map.get(key).cloned()
    }
}
