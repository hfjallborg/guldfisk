use crossbeam_channel::Sender;
use foldhash::{HashMap, HashMapExt};
use std::time::SystemTime;

pub struct Message {
    pub content: String,
    pub channel: String,
    pub timestamp: SystemTime,
}

impl std::clone::Clone for Message {
    fn clone(&self) -> Message {
        Message {
            content: self.content.clone(),
            channel: self.channel.clone(),
            timestamp: self.timestamp,
        }
    }
}

/// Stores the current subscribers for a channel
pub struct SubscriptionTable {
    map: HashMap<String, HashMap<u64, Sender<Message>>>,
}

impl Default for SubscriptionTable {
    fn default() -> Self {
        Self::new()
    }
}

impl SubscriptionTable {
    pub fn new() -> Self {
        SubscriptionTable {
            map: HashMap::new(),
        }
    }

    pub fn add(&mut self, channel: &str, tx: Sender<Message>, connection_id: u64) {
        self.map
            .entry(channel.to_string())
            .or_default()
            .insert(connection_id, tx);
    }

    pub fn get(&self, channel: &str) -> HashMap<u64, Sender<Message>> {
        match self.map.get(channel) {
            Some(v) => v.clone(),
            None => HashMap::new(),
        }
    }

    /// Removes a subscriber from the specified channel
    pub fn remove(&mut self, channel: &str, id: &u64) {
        if let Some(subscribers) = self.map.get_mut(channel) {
            subscribers.remove(id);
        }
    }

    /// Removes a subscriber (from their ID) from all channels
    pub fn remove_all(&mut self, connection_id: &u64) {
        for subscribers in self.map.values_mut() {
            subscribers.remove(connection_id);
        }
    }
}
