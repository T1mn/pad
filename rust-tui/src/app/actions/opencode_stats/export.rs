use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::process::Command;

pub(super) fn export_opencode_stats(project: &str, command: &OsString) -> io::Result<PathBuf> {
    let body = collect_stats_output(project, command)?;
    let path = super::path::opencode_stats_path(
        project,
        crate::paths::opencode_stats_dir().as_path(),
        super::path::current_unix_secs(),
    );
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, body.as_bytes())?;
    Ok(path)
}

fn collect_stats_output(project: &str, command: &OsString) -> io::Result<String> {
    let output = Command::new(command)
        .args([
            "stats",
            "--project",
            project,
            "--models",
            "10",
            "--tools",
            "10",
        ])
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(io::Error::other(if stderr.is_empty() {
            format!("opencode stats exited with {}", output.status)
        } else {
            stderr
        }));
    }

    let body = String::from_utf8_lossy(&output.stdout).to_string();
    if body.trim().is_empty() {
        Err(io::Error::other("opencode stats returned empty output"))
    } else {
        Ok(body)
    }
}
