use super::mode::ExportMode;
use super::path::opencode_export_path;
use std::io;
use std::path::PathBuf;

pub(super) fn export_opencode_session(
    session_id: &str,
    command: &str,
    mode: ExportMode,
) -> io::Result<PathBuf> {
    let body = run_opencode_export(session_id, command, mode)?;
    let path = opencode_export_path(
        session_id,
        crate::paths::opencode_exports_dir().as_path(),
        mode,
    );
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, body.as_bytes())?;
    Ok(path)
}

fn run_opencode_export(session_id: &str, command: &str, mode: ExportMode) -> io::Result<String> {
    let mut args = vec!["export", session_id];
    if matches!(mode, ExportMode::Sanitized) {
        args.push("--sanitize");
    }
    let output = super::super::opencode_cli::run_with_args(command, &args, None)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(io::Error::other(if stderr.is_empty() {
            format!("opencode export exited with {}", output.status)
        } else {
            stderr
        }));
    }

    let body = String::from_utf8_lossy(&output.stdout).to_string();
    if body.trim().is_empty() {
        return Err(io::Error::other("opencode export returned empty output"));
    }
    Ok(body)
}
