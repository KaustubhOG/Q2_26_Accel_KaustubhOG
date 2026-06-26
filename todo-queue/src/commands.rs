use crate::error::{AppError, Result};
use crate::models::Todo;
use crate::persistence::{self, State};
use std::path::Path;

pub fn add(state: &mut State, path: &Path, description: &str) -> Result<()> {
    let id = state.next_id;
    state.queue.enqueue(Todo::new(id, description));
    state.next_id += 1;
    persistence::save(path, state)?;
    println!("added: {}", description);
    Ok(())
}

pub fn list(state: &State) {
    if state.queue.is_empty() {
        println!("no pending tasks");
        return;
    }
    for (i, todo) in state.queue.iter().enumerate() {
        println!("[{}] #{} — {}", i + 1, todo.id, todo.description);
    }
}

pub fn done(state: &mut State, path: &Path) -> Result<()> {
    let todo = state.queue.dequeue().ok_or(AppError::EmptyQueue)?;
    persistence::save(path, state)?;
    println!("completed: #{} — {}", todo.id, todo.description);
    Ok(())
}
