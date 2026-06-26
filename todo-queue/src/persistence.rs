use crate::error::{AppError, Result};
use crate::models::{Store, Todo};
use crate::queue::Queue;
use borsh::BorshDeserialize;
use std::fs;
use std::path::Path;

pub struct State {
    pub queue: Queue<Todo>,
    pub next_id: u64,
}

pub fn load(path: &Path) -> Result<State> {
    if !path.exists() {
        return Ok(State { queue: Queue::new(), next_id: 1 });
    }
    let bytes = fs::read(path)?;
    let store = Store::try_from_slice(&bytes)
        .map_err(|e| AppError::Deserialize(e.to_string()))?;
    Ok(State { queue: Queue::from(store.items), next_id: store.next_id })
}

pub fn save(path: &Path, state: &State) -> Result<()> {
    let store = Store {
        next_id: state.next_id,
        items: state.queue.iter().cloned().collect(),
    };
    let bytes = borsh::to_vec(&store)
        .map_err(|e| AppError::Serialize(e.to_string()))?;

    // atomic write via tmp + rename
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, &bytes)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Todo;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn tmp_path() -> std::path::PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        std::path::PathBuf::from(format!("test_todos_{}_{}.bin", std::process::id(), n))
    }

    #[test]
    fn save_and_load_round_trip() {
        let path = tmp_path();
        let mut state = State { queue: Queue::new(), next_id: 1 };
        state.queue.enqueue(Todo::new(1, "task one"));
        state.queue.enqueue(Todo::new(2, "task two"));
        state.next_id = 3;
        save(&path, &state).unwrap();

        let loaded = load(&path).unwrap();
        let items: Vec<Todo> = loaded.queue.iter().cloned().collect();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].description, "task one");
        assert_eq!(items[1].description, "task two");
        assert_eq!(loaded.next_id, 3);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn load_missing_file_returns_empty_queue() {
        let path = tmp_path();
        let _ = fs::remove_file(&path);
        let state = load(&path).unwrap();
        assert!(state.queue.is_empty());
        assert_eq!(state.next_id, 1);
    }

    #[test]
    fn save_overwrites_previous_state() {
        let path = tmp_path();
        let mut state = State { queue: Queue::new(), next_id: 1 };
        state.queue.enqueue(Todo::new(1, "original"));
        state.next_id = 2;
        save(&path, &state).unwrap();

        state.queue.dequeue();
        save(&path, &state).unwrap();

        let loaded = load(&path).unwrap();
        assert!(loaded.queue.is_empty());
        assert_eq!(loaded.next_id, 2);

        let _ = fs::remove_file(&path);
    }
}
