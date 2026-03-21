use crate::fuzzy::fuzzy_select_directory;
use std::error::Error;
use std::process::Command;

/// Create a new tmux session using native fuzzy finder to select path
pub fn create_new_session_fuzzy() -> Result<(), Box<dyn Error>> {
    match fuzzy_select_directory() {
        Ok(Some(selected)) => {
            create_session_in_path(&selected)?;
        }
        Ok(None) => {}
        Err(e) => {
            return Err(format!("Failed to show directory picker: {}", e).into());
        }
    }
    Ok(())
}

/// Create a new tmux session in the given path
pub fn create_session_in_path(path: &str) -> Result<(), Box<dyn Error>> {
    let session_name = path
        .replace('/', "_")
        .replace('.', "_")
        .replace('~', "home");

    let check = Command::new("tmux")
        .args(["has-session", "-t", &session_name])
        .output()?;

    if check.status.success() {
        println!("Session '{}' already exists, attaching...", session_name);
        Command::new("tmux")
            .args(["switch-client", "-t", &session_name])
            .spawn()?;
    } else {
        println!("Creating new session '{}' in {}...", session_name, path);
        Command::new("tmux")
            .args(["new-session", "-d", "-s", &session_name, "-c", path])
            .output()?;
    }

    Ok(())
}
