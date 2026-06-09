use super::options::{clear_target_was_zoomed, should_restore_target_zoom, target_was_zoomed};
use super::pane::{ensure_pane_unzoomed, ensure_pane_zoomed, focus_pane, PaneInfo};
use super::run_tmux;

const HIDDEN_WINDOW_PREFIX: &str = "__pad_sider_";

pub(super) fn create_helper(target: &PaneInfo) -> Result<String, String> {
    let binary = std::env::current_exe().map_err(|err| err.to_string())?;
    let width = super::super::sizing::stored_or_default_width(&target.pane_id);
    let command = format!(
        "{} __internal pad-sider ui --cwd {} --target-pane {}",
        shell_single_quote(&binary.to_string_lossy()),
        shell_single_quote(&target.cwd.to_string_lossy()),
        shell_single_quote(&target.pane_id),
    );
    run_tmux(&[
        "split-window",
        "-P",
        "-F",
        "#{pane_id}",
        "-h",
        "-b",
        "-d",
        "-l",
        &width,
        "-t",
        &target.pane_id,
        &command,
    ])
    .map(|value| value.trim().to_string())
}

pub(super) fn hide_helper(target_pane: &str, helper_pane: &str) -> Result<(), String> {
    let restore_zoom = should_restore_target_zoom(target_was_zoomed(target_pane));
    ensure_pane_unzoomed(helper_pane)?;
    let window_name = hidden_window_name(helper_pane);
    run_tmux(&["break-pane", "-d", "-s", helper_pane, "-n", &window_name])?;
    focus_pane(target_pane)?;
    if restore_zoom {
        ensure_pane_zoomed(target_pane)?;
    }
    clear_target_was_zoomed(target_pane)?;
    Ok(())
}

pub(super) fn show_helper(helper_pane: &str, target_pane: &str) -> Result<(), String> {
    let width = super::super::sizing::stored_or_default_width(target_pane);
    run_tmux(&[
        "join-pane",
        "-d",
        "-h",
        "-b",
        "-l",
        &width,
        "-s",
        helper_pane,
        "-t",
        target_pane,
    ])
    .map(|_| ())
}

pub(super) fn focus_and_zoom_helper(helper_pane: &str) -> Result<(), String> {
    focus_pane(helper_pane)?;
    ensure_pane_zoomed(helper_pane)
}

fn hidden_window_name(helper_pane: &str) -> String {
    format!("{}{helper_pane}", HIDDEN_WINDOW_PREFIX).replace('%', "p")
}

fn shell_single_quote(value: &str) -> String {
    crate::shell_quote::single_quote(value)
}
