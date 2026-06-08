use std::path::PathBuf;

use super::{run_tmux, PANE_INFO_SEP};

#[derive(Debug, PartialEq, Eq)]
pub(super) struct PaneInfo {
    pub pane_id: String,
    pub window_id: String,
    pub command: String,
    pub cwd: PathBuf,
    pub zoomed: bool,
}

pub(super) fn pane_info(target_pane: &str) -> Result<PaneInfo, String> {
    let format = format!(
        "#{{pane_id}}{PANE_INFO_SEP}#{{session_name}}{PANE_INFO_SEP}#{{window_id}}{PANE_INFO_SEP}#{{pane_current_command}}{PANE_INFO_SEP}#{{pane_current_path}}{PANE_INFO_SEP}#{{window_zoomed_flag}}"
    );
    let output = run_tmux(&["display-message", "-p", "-t", target_pane, &format])?;
    parse_pane_info(output.trim())
}

fn parse_pane_info(raw: &str) -> Result<PaneInfo, String> {
    let parts: Vec<_> = raw.split(PANE_INFO_SEP).collect();
    if parts.len() != 6 {
        return Err(format!("unexpected pane info: {raw}"));
    }
    Ok(PaneInfo {
        pane_id: parts[0].to_string(),
        window_id: parts[2].to_string(),
        command: parts[3].to_string(),
        cwd: PathBuf::from(parts[4]),
        zoomed: parts[5] == "1",
    })
}

pub(super) fn panes_share_window(left: &str, right: &str) -> Result<bool, String> {
    let left_window = pane_info(left)?.window_id;
    let right_window = pane_info(right)?.window_id;
    Ok(left_window == right_window)
}

pub(super) fn pane_exists(pane_id: &str) -> bool {
    run_tmux(&["list-panes", "-a", "-F", "#{pane_id}"])
        .map(|output| output.lines().any(|line| line.trim() == pane_id))
        .unwrap_or(false)
}

pub(super) fn focus_pane(pane: &str) -> Result<(), String> {
    run_tmux(&["select-pane", "-t", pane]).map(|_| ())
}

pub(super) fn ensure_pane_zoomed(pane: &str) -> Result<(), String> {
    if !pane_info(pane)?.zoomed {
        run_tmux(&["resize-pane", "-Z", "-t", pane])?;
    }
    Ok(())
}

pub(super) fn ensure_pane_unzoomed(pane: &str) -> Result<(), String> {
    if pane_info(pane)?.zoomed {
        run_tmux(&["resize-pane", "-Z", "-t", pane])?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "pane_tests.rs"]
mod tests;
