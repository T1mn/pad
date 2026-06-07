use std::process::Command;

pub(super) fn tmux_status_value(target_session: &str) -> String {
    Command::new("tmux")
        .args(["show", "-v", "-t", target_session, "status"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

pub(super) fn apply_desired_status(
    desired_status: &str,
    current_status: &str,
    keep_source_status: Option<&str>,
    target_session: &str,
) -> Option<String> {
    let next_status = desired_status_override(desired_status, current_status, keep_source_status)?;

    let _ = Command::new("tmux")
        .args(["set", "-t", target_session, "status", &next_status])
        .output();
    Some(current_status.to_string())
}

pub(super) fn desired_status_override(
    desired_status: &str,
    current_status: &str,
    keep_source_status: Option<&str>,
) -> Option<String> {
    if current_status.is_empty() {
        return None;
    }

    match desired_status {
        "show" if current_status != "on" => Some("on".to_string()),
        "hide" if current_status != "off" => Some("off".to_string()),
        "keep" => keep_source_status
            .filter(|status| !status.is_empty() && *status != current_status)
            .map(str::to_string),
        _ => None,
    }
}
