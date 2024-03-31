use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use dashmap::DashMap;

#[derive(Debug, Clone)]
pub struct ItemHistory {
    // (item_id, site_id), timestamp
    items: HashMap<(i64, i32), i64>,
}

// (user_id, delivery_method) => ItemHistory
pub type ItemHistoryStorage = Arc<DashMap<(u64, i32), Arc<Mutex<ItemHistory>>>>;

impl ItemHistory {
    pub fn new() -> ItemHistory {
        Self {
            items: HashMap::new(),
        }
    }

    pub fn add_item(&mut self, id: i64, site_id: i32, timestamp: i64) -> bool {
        if !self.contains(id, site_id) {
            debug!("Adding id: {},{}, timestamp: {}", id, site_id, timestamp);
            self.items.insert((id, site_id), timestamp);
            true
        } else {
            false
        }
    }

    pub fn contains(&self, id: i64, site_id: i32) -> bool {
        self.items.contains_key(&(id, site_id))
    }

    pub fn purge_old(&mut self) {
        self.items
            .retain(|(_, _), timestamp| timestamp > &mut (chrono::Utc::now().timestamp() - 1000));
    }

    pub fn extend(&mut self, other: &Self) {
        self.items.extend(other.items.iter());
        self.purge_old()
    }
}

impl Default for ItemHistory {
    fn default() -> Self {
        Self::new()
    }
}
