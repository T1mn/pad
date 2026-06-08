mod ansi;
mod batch;

pub use ansi::strip_ansi;
use batch::{batch_capture_args, parse_batch_capture};
use std::collections::HashMap;
use std::process::Command;

pub(super) fn capture_pane_content(
    pane_id: &str,
    lines: usize,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
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
        Ok(strip_ansi(&String::from_utf8_lossy(&output.stdout)))
    } else {
        Err("Failed to capture pane".into())
    }
}

pub(super) fn capture_panes_content(
    pane_ids: &[String],
    lines: usize,
) -> Result<HashMap<String, String>, Box<dyn std::error::Error + Send + Sync>> {
    if pane_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let marker_prefix = format!("__PAD_CAPTURE_{}_", std::process::id());
    let output = Command::new("tmux")
        .args(batch_capture_args(pane_ids, lines, &marker_prefix))
        .output()?;

    if !output.status.success() {
        return Err("Failed to batch capture panes".into());
    }

    Ok(parse_batch_capture(
        &String::from_utf8_lossy(&output.stdout),
        pane_ids,
        &marker_prefix,
    ))
}
