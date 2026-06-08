use std::error::Error;
use std::process::Command;

/// Capture tmux pane content.
pub fn capture_pane(pane_id: &str, lines: usize) -> Result<String, Box<dyn Error>> {
    let output = Command::new("tmux")
        .args([
            "capture-pane",
            "-p",
            "-t",
            pane_id,
            "-S",
            &format!("-{}", lines),
        ])
        .output()?;

    if output.status.success() {
        Ok(crate::scanner::strip_ansi(&String::from_utf8_lossy(
            &output.stdout,
        )))
    } else {
        Err("Failed to capture pane".into())
    }
}
