use crate::app::App;
use std::error::Error;
use std::process::Command;

/// Create a new tmux session in the given path with an agent command.
/// After creation, switches the tmux client to the new session and installs
/// F12/Ctrl+Q bindings so the user can return to the pad session.
pub fn create_session_with_agent(
    app: &mut App,
    path: &str,
    agent_cmd: &str,
) -> Result<(), Box<dyn Error>> {
    let session_name = path.replace(['/', '.'], "_").replace('~', "home");
    let target_format = "#{session_name}:#{window_index} #{pane_id}";

    log_debug!(
        "session: create_session_with_agent path={} cmd={} session_name={}",
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
        "session: target_window={} target_pane={}",
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
        "session: pad_pane={:?} pad_win={:?} pad_session={:?}",
        pad_pane,
        pad_win,
        pad_session
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
    let status_restore_value = apply_desired_status(desired_status, &current_status, &session_name);
    app.saved_tmux_status = status_restore_value.clone();
    app.saved_tmux_status_target = status_restore_value.as_ref().map(|_| session_name.clone());

    // Install F12/Ctrl+Q bindings in the new session so user can return to pad
    if let (Some(pane_id), Some(win_target), Some(pad_session)) = (pad_pane, pad_win, pad_session) {
        let mut restore_parts = Vec::new();
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
        restore_parts.push(format!("tmux switch-client -t '{}'", pad_session));
        restore_parts.push(format!("tmux select-window -t '{}'", win_target));
        restore_parts.push(format!("tmux select-pane -t '{}'", pane_id));
        let return_cmd = restore_parts.join("; ");
        let bind_result = Command::new("tmux")
            .args(["bind-key", "-T", "root", "F12", "run-shell", &return_cmd])
            .output();
        log_debug!(
            "session: installed F12 return binding -> {} (result={:?})",
            return_cmd,
            bind_result.map(|o| o.status)
        );

        let _ = Command::new("tmux")
            .args(["bind-key", "-T", "root", "C-q", "run-shell", &return_cmd])
            .output();

        app.same_session_attached = true;
    } else {
        log_debug!("session: TMUX_PANE not set, skipping F12 binding install");
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
    let sw = Command::new("tmux")
        .args(["switch-client", "-t", &session_name])
        .output();
    log_debug!(
        "session: switch-client -t {} result={:?}",
        session_name,
        sw.map(|o| o.status)
    );

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
        .map(|line| line.to_string())
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
    target_session: &str,
) -> Option<String> {
    if current_status.is_empty() {
        return None;
    }

    match desired_status {
        "show" if current_status != "on" => {
            let _ = Command::new("tmux")
                .args(["set", "-t", target_session, "status", "on"])
                .output();
            Some(current_status.to_string())
        }
        "hide" if current_status != "off" => {
            let _ = Command::new("tmux")
                .args(["set", "-t", target_session, "status", "off"])
                .output();
            Some(current_status.to_string())
        }
        _ => None,
    }
}
