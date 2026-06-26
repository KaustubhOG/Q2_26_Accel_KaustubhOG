mod commands;
mod error;
mod models;
mod persistence;
mod queue;

use error::{AppError, Result};
use std::path::PathBuf;

fn data_path() -> PathBuf {
    PathBuf::from("todos.bin")
}

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = data_path();
    let mut state = persistence::load(&path)?;

    match args.as_slice() {
        [cmd, rest @ ..] if cmd == "add" => {
            let description = rest.join(" ");
            if description.is_empty() {
                return Err(AppError::BadArgs);
            }
            commands::add(&mut state, &path, &description)
        }
        [cmd] if cmd == "list" => {
            commands::list(&state);
            Ok(())
        }
        [cmd] if cmd == "done" => commands::done(&mut state, &path),
        _ => Err(AppError::BadArgs),
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
