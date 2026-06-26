use borsh::{BorshDeserialize, BorshSerialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct Todo {
    pub id: u64,
    pub description: String,
    pub created_at: u64,
}

impl Todo {
    pub fn new(id: u64, description: impl Into<String>) -> Self {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self { id, description: description.into(), created_at }
    }
}

// wire format: persists both the queue items and the id counter together
#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct Store {
    pub next_id: u64,
    pub items: Vec<Todo>,
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sets_fields() {
        let t = Todo::new(1, "test task");
        assert_eq!(t.id, 1);
        assert_eq!(t.description, "test task");
        assert!(t.created_at > 0);
    }

    #[test]
    fn borsh_round_trip() {
        let original = Todo::new(7, "round trip");
        let bytes = borsh::to_vec(&original).unwrap();
        let restored = Todo::try_from_slice(&bytes).unwrap();
        assert_eq!(restored.id, original.id);
        assert_eq!(restored.description, original.description);
        assert_eq!(restored.created_at, original.created_at);
    }

    #[test]
    fn store_round_trip() {
        let store = Store {
            next_id: 5,
            items: vec![Todo::new(3, "a"), Todo::new(4, "b")],
        };
        let bytes = borsh::to_vec(&store).unwrap();
        let restored = Store::try_from_slice(&bytes).unwrap();
        assert_eq!(restored.next_id, 5);
        assert_eq!(restored.items.len(), 2);
        assert_eq!(restored.items[1].description, "b");
    }
}