mod pane;

use std::process::Command;

use pane::{
    ensure_pane_unzoomed, ensure_pane_zoomed, focus_pane, pane_exists, pane_info,
    panes_share_window, PaneInfo,
};

const TARGET_OPTION: &str = "@pad_sider_pane";
const HELPER_TARGET_OPTION: &str = "@pad_sider_target";
const TARGET_WAS_ZOOMED_OPTION: &str = "@pad_sider_target_was_zoomed";
const HIDDEN_WINDOW_PREFIX: &str = "__pad_sider_";
const PANE_INFO_SEP: &str = "\x1f";

pub fn toggle(target_pane: &str) -> Result<(), String> {
    let target_pane = resolve_target_pane(target_pane)?;
    let target = pane_info(&target_pane)?;
    if !target.command.to_lowercase().contains("codex") {
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
            set_option(&target_pane, TARGET_OPTION, &helper_pane)?;
            set_option(&helper_pane, HELPER_TARGET_OPTION, &target_pane)?;
            focus_and_zoom_helper(&helper_pane)?;
        }
    }

    Ok(())
}

fn resolve_target_pane(invoked_pane: &str) -> Result<String, String> {
    if let Some(target_pane) = show_option(invoked_pane, HELPER_TARGET_OPTION)
        .filter(|value| !value.trim().is_empty())
        .filter(|pane| pane_exists(pane))
    {
        return Ok(target_pane);
    }
    Ok(invoked_pane.to_string())
}

fn helper_pane_for_target(target_pane: &str) -> Result<Option<String>, String> {
    let helper = show_option(target_pane, TARGET_OPTION).filter(|value| !value.trim().is_empty());
    match helper {
        Some(helper_pane) if pane_exists(&helper_pane) => Ok(Some(helper_pane)),
        Some(_) => {
            unset_option(target_pane, TARGET_OPTION)?;
            Ok(None)
        }
        None => Ok(None),
    }
}

fn create_helper(target: &PaneInfo) -> Result<String, String> {
    let binary = std::env::current_exe().map_err(|err| err.to_string())?;
    let width = super::sizing::stored_or_default_width(&target.pane_id);
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

fn hide_helper(target_pane: &str, helper_pane: &str) -> Result<(), String> {
    let restore_zoom =
        should_restore_target_zoom(show_option(target_pane, TARGET_WAS_ZOOMED_OPTION));
    ensure_pane_unzoomed(helper_pane)?;
    let window_name = hidden_window_name(helper_pane);
    run_tmux(&["break-pane", "-d", "-s", helper_pane, "-n", &window_name])?;
    focus_pane(target_pane)?;
    if restore_zoom {
        ensure_pane_zoomed(target_pane)?;
    }
    unset_option(target_pane, TARGET_WAS_ZOOMED_OPTION)?;
    Ok(())
}

fn show_helper(helper_pane: &str, target_pane: &str) -> Result<(), String> {
    let width = super::sizing::stored_or_default_width(target_pane);
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

fn focus_and_zoom_helper(helper_pane: &str) -> Result<(), String> {
    focus_pane(helper_pane)?;
    ensure_pane_zoomed(helper_pane)
}

fn remember_target_zoom(target_pane: &str, zoomed: bool) -> Result<(), String> {
    set_option(
        target_pane,
        TARGET_WAS_ZOOMED_OPTION,
        if zoomed { "1" } else { "0" },
    )
}

fn should_restore_target_zoom(value: Option<String>) -> bool {
    value.as_deref() == Some("1")
}

fn show_option(target: &str, key: &str) -> Option<String> {
    run_tmux(&["show-options", "-p", "-v", "-t", target, key])
        .ok()
        .map(|value| value.trim().to_string())
}

fn set_option(target: &str, key: &str, value: &str) -> Result<(), String> {
    run_tmux(&["set-option", "-p", "-t", target, key, value]).map(|_| ())
}

fn unset_option(target: &str, key: &str) -> Result<(), String> {
    run_tmux(&["set-option", "-p", "-t", target, "-u", key]).map(|_| ())
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

fn hidden_window_name(helper_pane: &str) -> String {
    format!("{}{helper_pane}", HIDDEN_WINDOW_PREFIX).replace('%', "p")
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r#"'\''"#))
}

#[cfg(test)]
mod tests {
    use super::super::sizing::default_width;
    use super::*;

    #[test]
    fn helper_uses_half_width() {
        assert_eq!(default_width(), "50%");
    }

    #[test]
    fn zoom_restore_option_only_restores_explicit_zoomed_targets() {
        assert!(should_restore_target_zoom(Some("1".into())));
        assert!(!should_restore_target_zoom(Some("0".into())));
        assert!(!should_restore_target_zoom(None));
    }
}
