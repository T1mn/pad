mod model;
mod scan;

use std::io;
use std::path::PathBuf;

pub use model::GrokThreadRef;

pub fn all_threads() -> io::Result<Vec<GrokThreadRef>> {
    scan::all_threads_at(&sessions_root()?)
}

pub fn thread_for_id(session_id: &str) -> io::Result<Option<GrokThreadRef>> {
    Ok(all_threads()?
        .into_iter()
        .find(|thread| thread.session_id == session_id))
}

fn sessions_root() -> io::Result<PathBuf> {
    if let Some(home) = std::env::var_os("GROK_HOME").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(home).join("sessions"));
    }
    dirs::home_dir()
        .map(|home| home.join(".grok").join("sessions"))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "home directory not found"))
}

#[cfg(test)]
mod tests;
