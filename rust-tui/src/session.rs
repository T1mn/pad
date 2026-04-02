use crate::app::App;
use std::error::Error;
use std::process::Command;

const PAD_RETURN_BINDING_MARKER: &str = "PAD_RETURN_BINDING=1;";

/// Create a new tmux session in the given path with an agent command.
/// After creation, switches the tmux client to the new session and installs
/// F12/Ctrl+Q bindings so the user can return to the pad session.
pub fn create_session_with_agent(
    app: &mut App,
    path: &str,
    agent_cmd: &str,
) -> Result<(), Box<dyn Error>> {
    let trace_id = crate::app::new_handoff_trace("create");
    app.same_session_trace_id = Some(trace_id.clone());
    let session_name = path.replace(['/', '.'], "_").replace('~', "home");
    let target_format = "#{session_name}:#{window_index} #{pane_id}";

    log_debug!(
        "handoff trace={} stage=create.begin path={} cmd={} session_name={}",
        trace_id,
        path,
        agent_cmd,
        session_name
    );

    let check = Command::new("tmux")
        .args(["has-session", "-t", &session_name])
        .output()?;

    let launch_after_attach = should_launch_after_attach(agent_cmd);
    let target_output = if check.status.success() {
        log_debug!(
            "session: session '{}' already exists, opening new window",
            session_name
        );
        // Session exists, open a new window. Gemini is launched after attach so it
        // can see a live client during terminal capability detection.
        let mut cmd = Command::new("tmux");
        cmd.args([
            "new-window",
            "-P",
            "-F",
            target_format,
            "-t",
            &session_name,
            "-c",
            path,
        ]);
        if !launch_after_attach {
            cmd.arg(agent_cmd);
        }
        let out = cmd.output()?;
        log_debug!(
            "session: new-window status={} stderr={}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
        out
    } else {
        log_debug!("session: creating new session '{}'", session_name);
        // Create new session. Gemini is launched after attach so it can see a
        // live client during terminal capability detection.
        let mut cmd = Command::new("tmux");
        cmd.args([
            "new-session",
            "-d",
            "-P",
            "-F",
            target_format,
            "-s",
            &session_name,
            "-c",
            path,
        ]);
        if !launch_after_attach {
            cmd.arg(agent_cmd);
        }
        let out = cmd.output()?;
        log_debug!(
            "session: new-session status={} stderr={}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
        out
    };

    if !target_output.status.success() {
        return Err(format!(
            "tmux create failed: {}",
            String::from_utf8_lossy(&target_output.stderr).trim()
        )
        .into());
    }

    let target_info = String::from_utf8_lossy(&target_output.stdout)
        .trim()
        .to_string();
    let mut target_parts = target_info.split_whitespace();
    let target_window = target_parts
        .next()
        .map(str::to_string)
        .unwrap_or_else(|| format!("{}:0", session_name));
    let target_pane = target_parts.next().map(str::to_string);

    log_debug!(
        "handoff trace={} stage=create.target_resolved target_window={} target_pane={}",
        trace_id,
        target_window,
        target_pane.as_deref().unwrap_or("-")
    );

    // Get the pad pane id so we can install a return binding
    let pad_pane = std::env::var("TMUX_PANE").ok();
    let pad_win = pad_pane.as_deref().and_then(|pane_id| {
        Command::new("tmux")
            .args([
                "display-message",
                "-p",
                "-t",
                pane_id,
                "#{session_name}:#{window_index}",
            ])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    });

    let pad_session = pad_pane.as_deref().and_then(|pane_id| {
        Command::new("tmux")
            .args(["display-message", "-p", "-t", pane_id, "#{session_name}"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    });

    log_debug!(
        "handoff trace={} stage=create.pad_context pad_pane={:?} pad_win={:?} pad_session={:?}",
        trace_id,
        pad_pane,
        pad_win,
        pad_session
    );

    let pad_client_tty = current_tmux_client_tty();
    let pad_client_snapshot = current_tmux_client_snapshot();
    log_debug!(
        "handoff trace={} stage=create.client_context client_tty={:?} snapshot={}",
        trace_id,
        pad_client_tty,
        pad_client_snapshot.as_deref().unwrap_or("-")
    );

    app.saved_tmux_bindings.clear();
    if let Some(line) = current_root_binding("F12") {
        app.saved_tmux_bindings.push(line);
    }
    if let Some(line) = current_root_binding("C-q") {
        app.saved_tmux_bindings.push(line);
    }

    let current_status = tmux_status_value(&session_name);
    let desired_status = app.config.desired_agent_style.status.as_str();
    let keep_source_status = if desired_status == "keep" {
        pad_session.as_deref().map(tmux_status_value)
    } else {
        None
    };
    let status_restore_value = apply_desired_status(
        desired_status,
        &current_status,
        keep_source_status.as_deref(),
        &session_name,
    );
    app.saved_tmux_status = status_restore_value.clone();
    app.saved_tmux_status_target = status_restore_value.as_ref().map(|_| session_name.clone());

    // Install F12/Ctrl+Q bindings in the new session so user can return to pad
    if let (Some(pane_id), Some(win_target), Some(pad_session)) = (pad_pane, pad_win, pad_session) {
        let mut restore_parts = Vec::new();
        restore_parts.push(shell_trace_log_cmd(
            &trace_id,
            "return.begin",
            &format!(
                "target_session={} target_window={} target_pane={} pad_session={} pad_window={} pad_pane={}",
                session_name, target_window, pane_id, pad_session, win_target, pane_id
            ),
        ));
        restore_parts.push(restore_binding_cmd(
            app.saved_tmux_bindings
                .iter()
                .find(|line| line.contains(" F12 "))
                .map(String::as_str),
            "F12",
        ));
        restore_parts.push(restore_binding_cmd(
            app.saved_tmux_bindings
                .iter()
                .find(|line| line.contains(" C-q "))
                .map(String::as_str),
            "C-q",
        ));
        if let Some(status) = status_restore_value.as_deref() {
            restore_parts.push(format!(
                "tmux set -t '{}' status '{}'",
                session_name, status
            ));
        }
        restore_parts.push(shell_trace_log_cmd(
            &trace_id,
            "return.before_switch",
            &format!("pad_session={} pad_window={} pad_pane={}", pad_session, win_target, pane_id),
        ));
        restore_parts.push(format!("tmux switch-client -t '{}'", pad_session));
        restore_parts.push(format!("tmux select-window -t '{}'", win_target));
        restore_parts.push(format!("tmux select-pane -t '{}'", pane_id));
        restore_parts.push(shell_trace_log_cmd(
            &trace_id,
            "return.after_select",
            &format!("pad_session={} pad_window={} pad_pane={}", pad_session, win_target, pane_id),
        ));
        let return_cmd = format!("{} {}", PAD_RETURN_BINDING_MARKER, restore_parts.join("; "));
        let run_shell_cmd = wrap_tmux_run_shell(&return_cmd);
        let bind_result = Command::new("tmux")
            .args(["bind-key", "-T", "root", "F12", "run-shell", &run_shell_cmd])
            .output();
        log_debug!(
            "handoff trace={} stage=create.bind_installed cmd={} result={:?}",
            trace_id,
            run_shell_cmd,
            bind_result.map(|o| o.status)
        );

        let _ = Command::new("tmux")
            .args(["bind-key", "-T", "root", "C-q", "run-shell", &run_shell_cmd])
            .output();

        app.same_session_attached = true;
        log_debug!("handoff trace={} stage=create.same_session_attached", trace_id);
    } else {
        log_debug!(
            "handoff trace={} stage=create.skip_return_binding reason=tmux_pane_missing",
            trace_id
        );
    }

    let _ = Command::new("tmux")
        .args(["select-window", "-t", &target_window])
        .output();
    if let Some(target_pane) = target_pane.as_deref() {
        let _ = Command::new("tmux")
            .args(["select-pane", "-t", target_pane])
            .output();
    }

    // Switch the tmux client to the target session
    let before_switch_snapshot = current_tmux_client_snapshot();
    log_debug!(
        "handoff trace={} stage=create.before_switch snapshot={}",
        trace_id,
        before_switch_snapshot.as_deref().unwrap_or("-")
    );

    let sw = Command::new("tmux")
        .args(["switch-client", "-t", &session_name])
        .output();
    log_debug!(
        "handoff trace={} stage=create.switch_client target_session={} result={:?}",
        trace_id,
        session_name,
        sw.map(|o| o.status)
    );

    let after_switch_snapshot = current_tmux_client_snapshot();
    log_debug!(
        "handoff trace={} stage=create.after_switch snapshot={}",
        trace_id,
        after_switch_snapshot.as_deref().unwrap_or("-")
    );

    show_return_hint(app.same_session_trace_id.as_deref(), pad_client_tty.as_deref());

    if launch_after_attach {
        if let Some(target_pane) = target_pane.as_deref() {
            launch_agent_after_attach(target_pane, agent_cmd);
        }
    }

    Ok(())
}

fn should_launch_after_attach(agent_cmd: &str) -> bool {
    matches!(agent_cmd.trim(), "gemini" | "gemini-cli")
}

fn launch_agent_after_attach(target_pane: &str, agent_cmd: &str) {
    let escaped_agent = shell_single_quote(agent_cmd);
    let escaped_pane = shell_single_quote(target_pane);
    let script = format!(
        "sleep 0.2; tmux send-keys -t {pane} C-c; tmux send-keys -t {pane} 'clear' Enter; tmux send-keys -t {pane} {agent} Enter",
        pane = escaped_pane,
        agent = escaped_agent
    );
    let result = Command::new("tmux")
        .args(["run-shell", "-b", &script])
        .output();
    log_debug!(
        "session: delayed launch target_pane={} cmd={} result={:?}",
        target_pane,
        agent_cmd,
        result.map(|o| o.status)
    );
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r#"'\''"#))
}

fn wrap_tmux_run_shell(script: &str) -> String {
    format!("sh -lc {}", shell_single_quote(script))
}

fn shell_trace_log_cmd(trace_id: &str, stage: &str, details: &str) -> String {
    let log_path = crate::paths::log_path().to_string_lossy().to_string();
    let message = format!("[handoff trace={}] stage={} {}", trace_id, stage, details);
    format!(
        "printf '[%s] %s\\n' \"$(date '+%H:%M:%S')\" {} >> {}",
        shell_single_quote(&message),
        shell_single_quote(&log_path)
    )
}

fn show_return_hint(trace_id: Option<&str>, client_tty: Option<&str>) {
    let popup_script = concat!(
        "printf '\\033[1mPAD\\033[0m\\n\\n'; ",
        "printf 'F12 or Ctrl+Q to return to PAD\\n\\n'; ",
        "printf 'This popup will close automatically in 3 seconds.\\n'; ",
        "sleep 3"
    );

    let before_popup_snapshot = current_tmux_client_snapshot();
    log_debug!(
        "handoff trace={} stage=create.return_hint.begin backend=tmux-display-popup client_tty={:?} snapshot={}",
        trace_id.unwrap_or("-"),
        client_tty,
        before_popup_snapshot.as_deref().unwrap_or("-")
    );

    let mut popup_cmd = Command::new("tmux");
    popup_cmd.args([
        "display-popup",
        "-E",
        "-w",
        "52",
        "-h",
        "7",
        "-T",
        "PAD",
    ]);
    if let Some(client_tty) = client_tty {
        popup_cmd.args(["-c", client_tty]);
    }
    popup_cmd.args(["sh", "-lc", popup_script]);
    let popup = popup_cmd.output();

    match popup {
        Ok(output) if output.status.success() => {
            let after_popup_snapshot = current_tmux_client_snapshot();
            log_debug!(
                "handoff trace={} stage=create.return_hint backend=tmux-display-popup delivered=true client_tty={:?} snapshot={}",
                trace_id.unwrap_or("-"),
                client_tty,
                after_popup_snapshot.as_deref().unwrap_or("-")
            );
            return;
        }
        Ok(output) => {
            log_debug!(
                "handoff trace={} stage=create.return_hint.error backend=tmux-display-popup client_tty={:?} status={} stderr={}",
                trace_id.unwrap_or("-"),
                client_tty,
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Err(err) => {
            log_debug!(
                "handoff trace={} stage=create.return_hint.error backend=tmux-display-popup client_tty={:?} err={}",
                trace_id.unwrap_or("-"),
                client_tty,
                err
            );
        }
    }

    log_debug!(
        "handoff trace={} stage=create.return_hint backend=tmux-display-message",
        trace_id.unwrap_or("-")
    );
    let _ = Command::new("tmux")
        .args([
            "display-message",
            "-d",
            "4000",
            "F12 or Ctrl+Q to return to PAD",
        ])
        .output();
}

fn current_root_binding(key: &str) -> Option<String> {
    let output = Command::new("tmux")
        .args(["list-keys", "-T", "root"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .find(|line| line.contains(&format!(" {} ", key)))
        .filter(|line| !is_pad_managed_binding(line))
        .map(|line| line.to_string())
}

fn current_tmux_client_tty() -> Option<String> {
    Command::new("tmux")
        .args(["display-message", "-p", "#{client_tty}"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|value| !value.is_empty())
}

fn current_tmux_client_snapshot() -> Option<String> {
    Command::new("tmux")
        .args([
            "display-message",
            "-p",
            "client=#{client_tty} session=#{session_name} window=#{window_index} pane=#{pane_id}",
        ])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|value| !value.is_empty())
}

fn is_pad_managed_binding(line: &str) -> bool {
    line.contains(PAD_RETURN_BINDING_MARKER)
        || (line.contains("run-shell")
            && line.contains("tmux select-window -t '")
            && line.contains("tmux select-pane -t '")
            && (line.contains("tmux switch-client -t '")
                || line.contains("[return] before_return_select")))
}

fn restore_binding_cmd(saved_binding: Option<&str>, key: &str) -> String {
    saved_binding
        .map(|line| format!("tmux {}", line))
        .unwrap_or_else(|| format!("tmux unbind-key -T root {}", key))
}

fn tmux_status_value(target_session: &str) -> String {
    Command::new("tmux")
        .args(["show", "-v", "-t", target_session, "status"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn apply_desired_status(
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

fn desired_status_override(
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

#[cfg(test)]
mod tests {
    use super::desired_status_override;

    #[test]
    fn keep_status_inherits_visible_status_from_pad_session() {
        assert_eq!(
            desired_status_override("keep", "off", Some("on")).as_deref(),
            Some("on")
        );
    }

    #[test]
    fn keep_status_noops_when_status_already_matches_pad_session() {
        assert_eq!(desired_status_override("keep", "on", Some("on")), None);
    }

    #[test]
    fn keep_status_noops_without_pad_status() {
        assert_eq!(desired_status_override("keep", "off", None), None);
        assert_eq!(desired_status_override("keep", "off", Some("")), None);
    }
}
