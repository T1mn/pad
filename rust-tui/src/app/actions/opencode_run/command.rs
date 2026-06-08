use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::process::Command;

pub(super) fn run_opencode_prompt(
    prompt: &str,
    session_id: Option<&str>,
    cwd: &Path,
    command: &OsString,
) -> io::Result<()> {
    let status = Command::new("tmux")
        .args(["new-window", "-c"])
        .arg(cwd)
        .arg(run_command(prompt, session_id, command))
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "tmux new-window exited with {status}"
        )))
    }
}

pub(in crate::app::actions) fn run_command(
    prompt: &str,
    session_id: Option<&str>,
    command: &OsString,
) -> String {
    let mut parts = vec![
        crate::codex_runtime::shell_single_quote(&command.to_string_lossy()),
        "run".to_string(),
    ];
    if let Some(session_id) = session_id {
        parts.push("--session".to_string());
        parts.push(crate::codex_runtime::shell_single_quote(session_id));
    }
    parts.push("--".to_string());
    parts.push(crate::codex_runtime::shell_single_quote(prompt));
    parts.join(" ")
}
