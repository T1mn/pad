use crate::app::App;
use crate::log_debug;
use crate::model::AgentPanel;
use crate::pty::attach_to_pane_pty;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Write};
use std::time::Duration;

const PAD_RETURN_BINDING_MARKER: &str = "PAD_RETURN_BINDING=1;";

fn summarize_log_text(text: &str) -> String {
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

fn run_tmux_logged(context: &str, args: Vec<String>) -> Option<std::process::Output> {
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

/// Install F12/C-q return bindings for same-session attach.
/// Snapshots zoom and status bar state, modifies them for the attach,
/// and encodes restoration into the return command.
fn install_return_bindings(app: &mut App, target_pane_id: &str, target_session: &str) -> bool {
    let trace_id = app
        .same_session_trace_id
        .clone()
        .unwrap_or_else(|| crate::app::new_handoff_trace("attach"));
    app.same_session_trace_id = Some(trace_id.clone());
    let pad_pane_id = match std::env::var("TMUX_PANE") {
        Ok(id) => id,
        Err(_) => {
            log_debug!(
                "handoff trace={} stage=attach.skip reason=tmux_pane_missing",
                trace_id
            );
            return false;
        }
    };

    log_debug!(
        "handoff trace={} stage=attach.begin target_pane={} target_session={} pad_pane={}",
        trace_id,
        target_pane_id,
        target_session,
        pad_pane_id
    );

    // Get pad's session:window_index for cross-window return
    let pad_win_target = std::process::Command::new("tmux")
        .args([
            "display-message",
            "-t",
            &pad_pane_id,
            "-p",
            "#{session_name}:#{window_index}",
        ])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    if pad_win_target.is_empty() {
        log_debug!(
            "install_return_bindings: pad_win_target empty, pad_pane_id={}",
            pad_pane_id
        );
        return false;
    }

    let pad_session = std::process::Command::new("tmux")
        .args([
            "display-message",
            "-t",
            &pad_pane_id,
            "-p",
            "#{session_name}",
        ])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    if pad_session.is_empty() {
        log_debug!(
            "install_return_bindings: pad_session empty, pad_pane_id={}",
            pad_pane_id
        );
        return false;
    }

    // --- Zoom: respect desired_agent_style.zoom config ---
    let zoom_info = std::process::Command::new("tmux")
        .args([
            "display-message",
            "-t",
            target_pane_id,
            "-p",
            "#{window_zoomed_flag} #{window_panes}",
        ])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let mut parts = zoom_info.split_whitespace();
    let already_zoomed = parts.next().unwrap_or("0") == "1";
    let pane_count: usize = parts.next().unwrap_or("1").parse().unwrap_or(1);
    let want_zoom = app.config.desired_agent_style.zoom == "auto";
    let should_zoom = want_zoom && pane_count > 1 && !already_zoomed;

    let restore_zoom_cmd = if should_zoom {
        // Do NOT zoom here — zoom happens after select-pane so user sees it instantly
        format!("tmux resize-pane -Z -t '{}'", target_pane_id)
    } else {
        String::new()
    };

    let saved_f12 = current_root_binding("F12");
    let saved_cq = current_root_binding("C-q");
    app.saved_tmux_bindings.clear();
    if let Some(line) = &saved_f12 {
        app.saved_tmux_bindings.push(line.clone());
    }
    if let Some(line) = &saved_cq {
        app.saved_tmux_bindings.push(line.clone());
    }

    log_debug!(
        "install_return_bindings: saved_bindings f12={} cq={}",
        saved_f12
            .as_deref()
            .map(summarize_log_text)
            .unwrap_or_else(|| "-".to_string()),
        saved_cq
            .as_deref()
            .map(summarize_log_text)
            .unwrap_or_else(|| "-".to_string())
    );

    // --- Status bar: respect desired_agent_style.status config ---
    let status_val = tmux_status_value(Some(target_session));
    let desired_status = app.config.desired_agent_style.status.as_str();
    let status_restore_value = apply_desired_status(desired_status, &status_val, target_session);
    app.saved_tmux_status = status_restore_value.clone();
    app.saved_tmux_status_target = status_restore_value
        .as_ref()
        .map(|_| target_session.to_string());
    let restore_status_cmd = status_restore_value
        .as_ref()
        .map(|status| format!("tmux set -t '{}' status '{}'", target_session, status))
        .unwrap_or_default();

    log_debug!(
        "install_return_bindings: target={} target_session={} panes={} zoomed={} should_zoom={} status={} desired_status={} status_restore={} pad_session={} pad_win={}",
        target_pane_id,
        target_session,
        pane_count,
        already_zoomed,
        should_zoom,
        status_val,
        desired_status,
        status_restore_value.as_deref().unwrap_or("-"),
        pad_session,
        pad_win_target
    );

    // Build return command: restore zoom + status, then navigate back to pad
    let mut restore_parts: Vec<String> = Vec::new();
    restore_parts.push(shell_log_cmd(&format!(
        "[handoff trace={}] stage=return.begin target_session={} target_pane={} pad_session={} pad_window={} pad_pane={}",
        trace_id, target_session, target_pane_id, pad_session, pad_win_target, pad_pane_id
    )));
    restore_parts.push(restore_binding_cmd(saved_f12.as_deref(), "F12"));
    restore_parts.push(restore_binding_cmd(saved_cq.as_deref(), "C-q"));
    if !restore_zoom_cmd.is_empty() {
        restore_parts.push(shell_log_cmd(&format!(
            "before_unzoom target_pane={}",
            target_pane_id
        )));
        restore_parts.push(restore_zoom_cmd);
        restore_parts.push(shell_log_cmd(&format!(
            "after_unzoom target_pane={}",
            target_pane_id
        )));
        restore_parts.push(wait_for_zoom_flag_cmd(
            target_pane_id,
            "0",
            "after_unzoom_wait",
        ));
    }
    if !restore_status_cmd.is_empty() {
        restore_parts.push(restore_status_cmd);
    }
    if target_session != pad_session {
        restore_parts.push(shell_log_cmd(&format!(
            "before_return_switch target_session={} pad_session={}",
            target_session, pad_session
        )));
        restore_parts.push(format!("tmux switch-client -t '{}'", pad_session));
        restore_parts.push(shell_log_cmd(&format!(
            "after_return_switch target_session={} pad_session={}",
            target_session, pad_session
        )));
    }
    restore_parts.push(shell_log_cmd(&format!(
        "before_return_select pad_window={} pad_pane={}",
        pad_win_target, pad_pane_id
    )));
    restore_parts.push(format!("tmux select-window -t '{}'", pad_win_target));
    restore_parts.push(format!("tmux select-pane -t '{}'", pad_pane_id));
    restore_parts.push(shell_log_cmd(&format!(
        "after_return_select pad_window={} pad_pane={}",
        pad_win_target, pad_pane_id
    )));

    let return_cmd = format!("{} {}", PAD_RETURN_BINDING_MARKER, restore_parts.join("; "));
    let run_shell_cmd = wrap_tmux_run_shell(&return_cmd);

    let _ = run_tmux_logged(
        "install_return_bindings.bind_f12",
        vec![
            "bind-key".to_string(),
            "-T".to_string(),
            "root".to_string(),
            "F12".to_string(),
            "run-shell".to_string(),
            run_shell_cmd.clone(),
        ],
    );
    let _ = run_tmux_logged(
        "install_return_bindings.bind_cq",
        vec![
            "bind-key".to_string(),
            "-T".to_string(),
            "root".to_string(),
            "C-q".to_string(),
            "run-shell".to_string(),
            run_shell_cmd.clone(),
        ],
    );

    log_debug!(
        "handoff trace={} stage=attach.return_cmd cmd={}",
        trace_id,
        run_shell_cmd
    );
    should_zoom
}

/// Clean up F12/C-q root bindings and restore status bar — safety net for pad quit/crash.
pub(super) fn restore_tmux_bindings(app: &mut App) {
    let trace_id = app
        .same_session_trace_id
        .clone()
        .unwrap_or_else(|| "-".to_string());
    let saved_f12 = app
        .saved_tmux_bindings
        .iter()
        .find(|line| line.contains(" F12 "))
        .cloned();
    let saved_cq = app
        .saved_tmux_bindings
        .iter()
        .find(|line| line.contains(" C-q "))
        .cloned();

    let _ = std::process::Command::new("sh")
        .args(["-lc", &restore_binding_cmd(saved_f12.as_deref(), "F12")])
        .output();
    let _ = std::process::Command::new("sh")
        .args(["-lc", &restore_binding_cmd(saved_cq.as_deref(), "C-q")])
        .output();

    if let (Some(status), Some(target)) = (
        app.saved_tmux_status.as_deref(),
        app.saved_tmux_status_target.as_deref(),
    ) {
        let _ = std::process::Command::new("tmux")
            .args(["set", "-t", target, "status", status])
            .output();
    }

    log_debug!(
        "handoff trace={} stage=restore_tmux_bindings restored root bindings and status",
        trace_id
    );
    app.saved_tmux_bindings.clear();
    app.saved_tmux_status = None;
    app.saved_tmux_status_target = None;
    app.same_session_trace_id = None;
}

fn current_root_binding(key: &str) -> Option<String> {
    let output = std::process::Command::new("tmux")
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

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn wrap_tmux_run_shell(script: &str) -> String {
    format!("sh -lc {}", shell_single_quote(script))
}

fn shell_log_cmd(message: &str) -> String {
    let log_path = crate::paths::log_path().to_string_lossy().to_string();
    format!(
        "printf '[%s] %s\\n' \"$(date '+%H:%M:%S')\" {} >> {}",
        shell_single_quote(&format!("[return] {}", message)),
        shell_single_quote(&log_path)
    )
}

fn wait_for_zoom_flag_cmd(target_pane_id: &str, expected_zoomed: &str, label: &str) -> String {
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

fn tmux_status_value(target_session: Option<&str>) -> String {
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

fn current_tmux_session() -> Option<String> {
    let output = std::process::Command::new("tmux")
        .args(["display-message", "-p", "#{session_name}"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub(super) fn current_tmux_pane_id() -> Option<String> {
    let output = std::process::Command::new("tmux")
        .args(["display-message", "-p", "#{pane_id}"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn current_tmux_window_target() -> Option<String> {
    let output = std::process::Command::new("tmux")
        .args(["display-message", "-p", "#{session_name}:#{window_index}"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn tmux_target_snapshot(target: &str) -> Option<String> {
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

pub(super) fn handle_attach(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    if let Some(panel) = app.selected_panel() {
        let panel = panel.clone();
        log_debug!(
            "attach: pane_id={} agent={} session={} window={}",
            panel.pane_id,
            panel.agent_type,
            panel.session,
            panel.window_index
        );

        // Clean up any stale temporary bindings before installing a fresh handoff.
        if !app.saved_tmux_bindings.is_empty() || app.same_session_attached {
            restore_tmux_bindings(app);
            app.same_session_attached = false;
        }

        // Detect if target pane is in the same tmux session
        let current_session = std::env::var("TMUX_PANE")
            .ok()
            .and_then(|_| current_tmux_session());

        log_debug!(
            "attach: current_session={} target_session={} current_window={} current_pane={}",
            current_session.as_deref().unwrap_or("-"),
            panel.session,
            current_tmux_window_target().as_deref().unwrap_or("-"),
            current_tmux_pane_id().as_deref().unwrap_or("-")
        );

        if let Some(current_session) = current_session.as_deref() {
            let cross_session = current_session != panel.session;
            if handoff_to_tmux_client(app, &panel, current_session, cross_session) {
                return Ok(());
            }

            if !cross_session {
                log_debug!("attach.same_session: handoff failed, leaving pad in place");
                app.dirty = true;
                return Ok(());
            }

            log_debug!(
                "attach.cross_session: client handoff failed current_session={} target_session={}, falling back to PTY",
                current_session,
                panel.session
            );
        }

        attach_via_pty(terminal, app, &panel)?;
    } else {
        log_debug!("attach: no panel selected");
    }
    Ok(())
}

fn handoff_to_tmux_client(
    app: &mut App,
    panel: &AgentPanel,
    current_session: &str,
    cross_session: bool,
) -> bool {
    let target_window = format!("{}:{}", panel.session, panel.window_index);
    let prefix = if cross_session {
        "attach.cross_session"
    } else {
        "attach.same_session"
    };

    log_debug!(
        "{}: start target_window={} target_pane={} current_session={} current_window={} current_pane={} target_snapshot={}",
        prefix,
        target_window,
        panel.pane_id,
        current_session,
        current_tmux_window_target().as_deref().unwrap_or("-"),
        current_tmux_pane_id().as_deref().unwrap_or("-"),
        tmux_target_snapshot(&panel.pane_id).as_deref().unwrap_or("-")
    );

    let should_zoom = install_return_bindings(app, &panel.pane_id, &panel.session);

    if cross_session
        && !run_tmux_success(
            "attach.cross_session.switch_client",
            vec![
                "switch-client".to_string(),
                "-t".to_string(),
                panel.session.clone(),
            ],
        )
    {
        log_debug!(
            "attach.cross_session: switch-client failed target_session={}",
            panel.session
        );
        restore_tmux_bindings(app);
        return false;
    }

    if cross_session {
        log_debug!(
            "attach.cross_session: after switch-client current_window={} current_pane={} target_snapshot={}",
            current_tmux_window_target().as_deref().unwrap_or("-"),
            current_tmux_pane_id().as_deref().unwrap_or("-"),
            tmux_target_snapshot(&panel.pane_id).as_deref().unwrap_or("-")
        );
    }

    if !run_tmux_success(
        &format!("{}.select_window", prefix),
        vec![
            "select-window".to_string(),
            "-t".to_string(),
            target_window.clone(),
        ],
    ) {
        log_debug!(
            "{}: select-window failed target_window={}",
            prefix,
            target_window
        );
        restore_tmux_bindings(app);
        return false;
    }

    log_debug!(
        "{}: after select-window current_window={} current_pane={} target_snapshot={}",
        prefix,
        current_tmux_window_target().as_deref().unwrap_or("-"),
        current_tmux_pane_id().as_deref().unwrap_or("-"),
        tmux_target_snapshot(&panel.pane_id)
            .as_deref()
            .unwrap_or("-")
    );

    if !run_tmux_success(
        &format!("{}.select_pane", prefix),
        vec![
            "select-pane".to_string(),
            "-t".to_string(),
            panel.pane_id.clone(),
        ],
    ) {
        log_debug!(
            "{}: select-pane failed target_pane={}",
            prefix,
            panel.pane_id
        );
        restore_tmux_bindings(app);
        return false;
    }

    log_debug!(
        "{}: after select-pane current_window={} current_pane={} target_snapshot={}",
        prefix,
        current_tmux_window_target().as_deref().unwrap_or("-"),
        current_tmux_pane_id().as_deref().unwrap_or("-"),
        tmux_target_snapshot(&panel.pane_id)
            .as_deref()
            .unwrap_or("-")
    );

    if should_zoom
        && !run_tmux_success(
            &format!("{}.resize_zoom", prefix),
            vec![
                "resize-pane".to_string(),
                "-Z".to_string(),
                "-t".to_string(),
                panel.pane_id.clone(),
            ],
        )
    {
        log_debug!(
            "{}: resize-pane failed target_pane={}",
            prefix,
            panel.pane_id
        );
        restore_tmux_bindings(app);
        return false;
    }

    if should_zoom {
        log_debug!(
            "{}: after resize-pane current_window={} current_pane={} target_snapshot={}",
            prefix,
            current_tmux_window_target().as_deref().unwrap_or("-"),
            current_tmux_pane_id().as_deref().unwrap_or("-"),
            tmux_target_snapshot(&panel.pane_id)
                .as_deref()
                .unwrap_or("-")
        );
    }

    app.same_session_attached = true;
    log_debug!(
        "{}: handoff complete target_window={} target_pane={} should_zoom={} target_snapshot={}",
        prefix,
        target_window,
        panel.pane_id,
        should_zoom,
        tmux_target_snapshot(&panel.pane_id)
            .as_deref()
            .unwrap_or("-")
    );
    app.dirty = true;
    true
}

fn attach_via_pty(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    panel: &AgentPanel,
) -> io::Result<()> {
    let status_before = tmux_status_value(Some(&panel.session));
    let desired_status = app.config.desired_agent_style.status.as_str();
    let status_restore_value = apply_desired_status(desired_status, &status_before, &panel.session);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    print!("\x1b[2J\x1b[H");
    println!(
        "Attaching to {} @ {} (window {})",
        panel.agent_type, panel.pane_id, panel.window_index
    );
    println!("Press F12, Ctrl+Q, or Ctrl+B then d to return to pad\n");
    io::stdout().flush()?;

    std::thread::sleep(Duration::from_millis(100));

    match attach_to_pane_pty(panel) {
        Ok(()) => {
            log_debug!("attach: detached normally from pane_id={}", panel.pane_id);
        }
        Err(e) => {
            log_debug!("attach: ERROR pane_id={} err={}", panel.pane_id, e);
            println!("Attach error: {}", e);
            println!("Press any key to continue...");
            io::stdout().flush()?;
            let _ = crossterm::event::read();
        }
    }

    print!("\x1b[2J\x1b[H");
    io::stdout().flush()?;

    enable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture
    )?;

    terminal.clear()?;

    if let Some(status) = status_restore_value.as_deref() {
        let _ = std::process::Command::new("tmux")
            .args(["set", "-t", &panel.session, "status", status])
            .output();
    }

    app.refresh_after_attach = true;
    app.dirty = true;
    Ok(())
}

fn run_tmux_success(context: &str, args: Vec<String>) -> bool {
    run_tmux_logged(context, args)
        .map(|output| output.status.success())
        .unwrap_or(false)
}
