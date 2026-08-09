//! Durable native-terminal layout state.
//!
//! Only declarative launch/layout metadata is stored. PTY processes, screen
//! contents, epochs, controller queues, and exit state are deliberately
//! runtime-only and are recreated on the next PAD launch.

use std::fs;
use std::fs::OpenOptions;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::app::TerminalWorkspace;

const TERMINAL_WORKSPACE_VERSION: u32 = 1;

#[derive(Deserialize, Serialize)]
struct TerminalWorkspaceFile {
    version: u32,
    workspace: TerminalWorkspace,
}

pub fn load() -> io::Result<Option<TerminalWorkspace>> {
    load_from_path(&crate::paths::terminal_workspace_path())
}

pub fn save(workspace: &TerminalWorkspace) -> io::Result<()> {
    save_to_path(&crate::paths::terminal_workspace_path(), workspace)
}

/// Moves an unreadable or unsupported workspace aside without overwriting an
/// earlier recovery file. PAD must only create a fresh workspace after this
/// succeeds, so a newer schema is never silently destroyed by an older build.
pub fn quarantine_invalid() -> io::Result<Option<PathBuf>> {
    quarantine_invalid_at(&crate::paths::terminal_workspace_path())
}

fn load_from_path(path: &Path) -> io::Result<Option<TerminalWorkspace>> {
    let body = match fs::read_to_string(path) {
        Ok(body) => body,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let mut stored: TerminalWorkspaceFile =
        serde_json::from_str(&body).map_err(invalid_workspace)?;
    if stored.version != TERMINAL_WORKSPACE_VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "unsupported terminal workspace version {} (expected {})",
                stored.version, TERMINAL_WORKSPACE_VERSION
            ),
        ));
    }
    stored
        .workspace
        .normalize_after_restore()
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidData, message))?;
    Ok(Some(stored.workspace))
}

fn save_to_path(path: &Path, workspace: &TerminalWorkspace) -> io::Result<()> {
    workspace
        .validate()
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;
    let stored = TerminalWorkspaceFile {
        version: TERMINAL_WORKSPACE_VERSION,
        workspace: workspace.clone(),
    };
    let mut body = serde_json::to_string_pretty(&stored).map_err(invalid_workspace)?;
    body.push('\n');
    crate::atomic_file::write_private(path, body)
}

fn quarantine_invalid_at(path: &Path) -> io::Result<Option<PathBuf>> {
    if !path.exists() {
        return Ok(None);
    }
    for suffix in 0..1000 {
        let candidate = quarantine_candidate(path, suffix);
        let placeholder = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        };
        drop(placeholder);
        match fs::rename(path, &candidate) {
            Ok(()) => {
                sync_parent_directory(path)?;
                return Ok(Some(candidate));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound && !path.exists() => {
                let _ = fs::remove_file(&candidate);
                return Ok(None);
            }
            Err(error) => {
                let _ = fs::remove_file(&candidate);
                return Err(error);
            }
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "too many quarantined terminal workspace files",
    ))
}

fn sync_parent_directory(path: &Path) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("terminal workspace path has no parent directory"))?;
    fs::File::open(parent)?.sync_all()
}

fn quarantine_candidate(path: &Path, suffix: usize) -> PathBuf {
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("terminal-workspace");
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| format!(".{value}"))
        .unwrap_or_default();
    let numbered = if suffix == 0 {
        String::new()
    } else {
        format!(".{suffix}")
    };
    path.with_file_name(format!("{stem}.invalid{numbered}{extension}"))
}

fn invalid_workspace(error: serde_json::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}

#[cfg(test)]
#[path = "terminal_workspace_tests.rs"]
pub(crate) mod tests;
