mod helper;
mod options;
mod pane;

use std::process::Command;

use helper::{create_helper, focus_and_zoom_helper, hide_helper, show_helper};
use options::{
    helper_pane_for_target, remember_helper_for_target, remember_target_for_helper,
    remember_target_zoom, resolve_target_pane,
};
use pane::{pane_info, panes_share_window};

const PANE_INFO_SEP: &str = "\x1f";

pub fn toggle(target_pane: &str) -> Result<(), String> {
    let target_pane = resolve_target_pane(target_pane);
    let target = pane_info(&target_pane)?;
    if !is_codex_command(&target.command) {
        return Ok(());
    }

    let helper = helper_pane_for_target(&target_pane)?;

    match helper {
        Some(helper_pane) if panes_share_window(&target_pane, &helper_pane)? => {
            hide_helper(&target_pane, &helper_pane)?;
        }
        Some(helper_pane) => {
            remember_target_zoom(&target_pane, pane_info(&target_pane)?.zoomed)?;
            show_helper(&helper_pane, &target_pane)?;
            focus_and_zoom_helper(&helper_pane)?;
        }
        None => {
            remember_target_zoom(&target_pane, target.zoomed)?;
            let helper_pane = create_helper(&target)?;
            remember_helper_for_target(&target_pane, &helper_pane)?;
            remember_target_for_helper(&helper_pane, &target_pane)?;
            focus_and_zoom_helper(&helper_pane)?;
        }
    }

    Ok(())
}

fn run_tmux(args: &[&str]) -> Result<String, String> {
    let output = Command::new("tmux")
        .args(args)
        .output()
        .map_err(|err| format!("tmux {}: {err}", args.join(" ")))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(format!(
            "tmux {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn is_codex_command(command: &str) -> bool {
    command
        .as_bytes()
        .windows("codex".len())
        .any(|window| window.eq_ignore_ascii_case(b"codex"))
}

#[cfg(test)]
#[path = "tmux_codex_tests.rs"]
mod tests;
