use crate::log_debug;

pub(super) fn summarize_log_text(text: &str) -> String {
    let single_line = text.trim().replace('\n', "\\n").replace('\r', "\\r");
    if single_line.is_empty() {
        return "-".to_string();
    }

    let mut shortened: String = single_line.chars().take(160).collect();
    if single_line.chars().count() > 160 {
        shortened.push('…');
    }
    shortened
}

pub(super) fn run_tmux_logged(context: &str, args: Vec<String>) -> Option<std::process::Output> {
    log_debug!("tmux:{}: cmd=tmux {}", context, args.join(" "));

    let output = std::process::Command::new("tmux")
        .args(args.iter().map(String::as_str))
        .output()
        .ok()?;

    log_debug!(
        "tmux:{}: exit={} stdout={} stderr={}",
        context,
        output.status,
        summarize_log_text(&String::from_utf8_lossy(&output.stdout)),
        summarize_log_text(&String::from_utf8_lossy(&output.stderr))
    );

    Some(output)
}

pub(super) fn current_tmux_session() -> Option<String> {
    let output = std::process::Command::new("tmux")
        .args(["display-message", "-p", "#{session_name}"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub(crate) fn current_tmux_pane_id() -> Option<String> {
    let output = std::process::Command::new("tmux")
        .args(["display-message", "-p", "#{pane_id}"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub(super) fn current_tmux_window_target() -> Option<String> {
    let output = std::process::Command::new("tmux")
        .args(["display-message", "-p", "#{session_name}:#{window_index}"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub(super) fn tmux_target_snapshot(target: &str) -> Option<String> {
    let output = std::process::Command::new("tmux")
        .args([
            "display-message",
            "-t",
            target,
            "-p",
            "window=#{session_name}:#{window_index} pane=#{pane_id} active=#{pane_active} zoomed=#{window_zoomed_flag} layout=#{window_layout} visible=#{window_visible_layout}",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub(super) fn run_tmux_success(context: &str, args: Vec<String>) -> bool {
    run_tmux_logged(context, args)
        .map(|output| output.status.success())
        .unwrap_or(false)
}

pub(super) fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

pub(super) fn wrap_tmux_run_shell(script: &str) -> String {
    format!("sh -lc {}", shell_single_quote(script))
}

pub(super) fn shell_log_cmd(message: &str) -> String {
    let log_path = crate::paths::log_path().to_string_lossy().to_string();
    format!(
        "printf '[%s] %s\\n' \"$(date '+%H:%M:%S')\" {} >> {}",
        shell_single_quote(&format!("[return] {}", message)),
        shell_single_quote(&log_path)
    )
}

pub(super) fn wait_for_zoom_flag_cmd(
    target_pane_id: &str,
    expected_zoomed: &str,
    label: &str,
) -> String {
    let log_path = crate::paths::log_path().to_string_lossy().to_string();
    format!(
        concat!(
            "_pad_wait_i=0; _pad_zoom=''; ",
            "while [ $_pad_wait_i -lt 30 ]; do ",
            "_pad_zoom=$(tmux display-message -t {} -p '#{{window_zoomed_flag}}' 2>/dev/null | tr -d '\\r\\n'); ",
            "[ \"$_pad_zoom\" = {} ] && break; ",
            "_pad_wait_i=$((_pad_wait_i + 1)); ",
            "sleep 0.01; ",
            "done; ",
            "printf '%s\\n' \"[return] {} target_pane={} zoomed=${{_pad_zoom:-?}} tries=${{_pad_wait_i}}\" >> {}"
        ),
        shell_single_quote(target_pane_id),
        shell_single_quote(expected_zoomed),
        label,
        target_pane_id,
        shell_single_quote(&log_path)
    )
}

pub(super) fn tmux_status_value(target_session: Option<&str>) -> String {
    let mut cmd = std::process::Command::new("tmux");
    cmd.arg("show").arg("-v");
    if let Some(target) = target_session {
        cmd.args(["-t", target]);
    }
    cmd.arg("status");
    cmd.output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

pub(super) fn apply_desired_status(
    desired_status: &str,
    current_status: &str,
    target_session: &str,
) -> Option<String> {
    if current_status.is_empty() {
        return None;
    }

    match desired_status {
        "show" if current_status != "on" => {
            let _ = std::process::Command::new("tmux")
                .args(["set", "-t", target_session, "status", "on"])
                .output();
            Some(current_status.to_string())
        }
        "hide" if current_status != "off" => {
            let _ = std::process::Command::new("tmux")
                .args(["set", "-t", target_session, "status", "off"])
                .output();
            Some(current_status.to_string())
        }
        _ => None,
    }
}
