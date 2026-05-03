use std::path::PathBuf;
use std::process::Command;

const TARGET_OPTION: &str = "@pad_sider_pane";
const HELPER_TARGET_OPTION: &str = "@pad_sider_target";
const HIDDEN_WINDOW_PREFIX: &str = "__pad_sider_";

pub fn toggle(target_pane: &str) -> Result<(), String> {
    let target_pane = resolve_target_pane(target_pane)?;
    let target = pane_info(&target_pane)?;
    if !target.command.to_lowercase().contains("codex") {
        return Ok(());
    }

    let helper = helper_pane_for_target(&target_pane)?;

    match helper {
        Some(helper_pane) if panes_share_window(&target_pane, &helper_pane)? => {
            hide_helper(&helper_pane)?;
        }
        Some(helper_pane) => {
            show_helper(&helper_pane, &target_pane)?;
            focus_helper(&helper_pane)?;
        }
        None => {
            let helper_pane = create_helper(&target)?;
            set_option(&target_pane, TARGET_OPTION, &helper_pane)?;
            set_option(&helper_pane, HELPER_TARGET_OPTION, &target_pane)?;
            focus_helper(&helper_pane)?;
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

struct PaneInfo {
    pane_id: String,
    window_id: String,
    command: String,
    cwd: PathBuf,
}

fn pane_info(target_pane: &str) -> Result<PaneInfo, String> {
    let output = run_tmux(&[
        "display-message",
        "-p",
        "-t",
        target_pane,
        "#{pane_id}|#{session_name}|#{window_id}|#{pane_current_command}|#{pane_current_path}",
    ])?;
    let parts: Vec<_> = output.trim().split('|').collect();
    if parts.len() != 5 {
        return Err(format!("unexpected pane info: {output}"));
    }
    Ok(PaneInfo {
        pane_id: parts[0].to_string(),
        window_id: parts[2].to_string(),
        command: parts[3].to_string(),
        cwd: PathBuf::from(parts[4]),
    })
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

fn hide_helper(helper_pane: &str) -> Result<(), String> {
    let window_name = hidden_window_name(helper_pane);
    run_tmux(&["break-pane", "-d", "-s", helper_pane, "-n", &window_name]).map(|_| ())
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

fn focus_helper(helper_pane: &str) -> Result<(), String> {
    run_tmux(&["select-pane", "-t", helper_pane]).map(|_| ())
}

fn panes_share_window(left: &str, right: &str) -> Result<bool, String> {
    let left_window = pane_info(left)?.window_id;
    let right_window = pane_info(right)?.window_id;
    Ok(left_window == right_window)
}

fn pane_exists(pane_id: &str) -> bool {
    run_tmux(&["list-panes", "-a", "-F", "#{pane_id}"])
        .map(|output| output.lines().any(|line| line.trim() == pane_id))
        .unwrap_or(false)
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

    #[test]
    fn helper_uses_half_width() {
        assert_eq!(default_width(), "50%");
    }
}
