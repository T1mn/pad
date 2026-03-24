use crate::app::App;
use crate::app::state::Mode;
use crate::log_debug;
use crate::pty::attach_to_pane_pty;
use crate::relay;
use crate::session;
use crate::ui;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Write};
use std::time::{Duration, Instant};

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
fn install_return_bindings(app: &mut App, target_pane_id: &str, _session: &str) -> bool {
    let pad_pane_id = match std::env::var("TMUX_PANE") {
        Ok(id) => id,
        Err(_) => {
            log_debug!("install_return_bindings: TMUX_PANE not set");
            return false;
        }
    };

    log_debug!(
        "install_return_bindings: start target_pane={} pad_pane={}",
        target_pane_id,
        pad_pane_id
    );

    // Get pad's session:window_index for cross-window return
    let pad_win_target = std::process::Command::new("tmux")
        .args(["display-message", "-t", &pad_pane_id, "-p", "#{session_name}:#{window_index}"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    if pad_win_target.is_empty() {
        log_debug!("install_return_bindings: pad_win_target empty, pad_pane_id={}", pad_pane_id);
        return false;
    }

    // --- Zoom: respect desired_agent_style.zoom config ---
    let zoom_info = std::process::Command::new("tmux")
        .args(["display-message", "-t", target_pane_id, "-p", "#{window_zoomed_flag} #{window_panes}"])
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
    let status_val = tmux_status_value();
    let desired_status = app.config.desired_agent_style.status.as_str();
    let status_restore_value = apply_desired_status(desired_status, &status_val);
    app.saved_tmux_status = status_restore_value.clone();
    let restore_status_cmd = status_restore_value
        .as_ref()
        .map(|status| format!("tmux set status '{}'", status))
        .unwrap_or_default();

    log_debug!(
        "install_return_bindings: target={} panes={} zoomed={} should_zoom={} status={} desired_status={} status_restore={} pad_win={}",
        target_pane_id,
        pane_count,
        already_zoomed,
        should_zoom,
        status_val,
        desired_status,
        status_restore_value.as_deref().unwrap_or("-"),
        pad_win_target
    );

    // Build return command: restore zoom + status, then navigate back to pad
    let mut restore_parts: Vec<String> = Vec::new();
    restore_parts.push(restore_binding_cmd(saved_f12.as_deref(), "F12"));
    restore_parts.push(restore_binding_cmd(saved_cq.as_deref(), "C-q"));
    if !restore_zoom_cmd.is_empty() {
        restore_parts.push(shell_log_cmd(&format!("before_unzoom target_pane={}", target_pane_id)));
        restore_parts.push(restore_zoom_cmd);
        restore_parts.push(shell_log_cmd(&format!("after_unzoom target_pane={}", target_pane_id)));
        restore_parts.push(wait_for_zoom_flag_cmd(target_pane_id, "0", "after_unzoom_wait"));
    }
    if !restore_status_cmd.is_empty() {
        restore_parts.push(restore_status_cmd);
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

    let return_cmd = restore_parts.join("; ");

    let _ = run_tmux_logged(
        "install_return_bindings.bind_f12",
        vec![
            "bind-key".to_string(),
            "-T".to_string(),
            "root".to_string(),
            "F12".to_string(),
            "run-shell".to_string(),
            return_cmd.clone(),
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
            return_cmd.clone(),
        ],
    );

    log_debug!("install_return_bindings: return_cmd={}", return_cmd);
    should_zoom
}

/// Clean up F12/C-q root bindings and restore status bar — safety net for pad quit/crash.
pub fn restore_tmux_bindings(app: &mut App) {
    let saved_f12 = app.saved_tmux_bindings.iter().find(|line| line.contains(" F12 ")).cloned();
    let saved_cq = app.saved_tmux_bindings.iter().find(|line| line.contains(" C-q ")).cloned();

    let _ = std::process::Command::new("sh")
        .args(["-lc", &restore_binding_cmd(saved_f12.as_deref(), "F12")])
        .output();
    let _ = std::process::Command::new("sh")
        .args(["-lc", &restore_binding_cmd(saved_cq.as_deref(), "C-q")])
        .output();

    if let Some(status) = app.saved_tmux_status.as_deref() {
        let _ = std::process::Command::new("tmux")
            .args(["set", "status", status])
            .output();
    }

    log_debug!("restore_tmux_bindings: restored root bindings and status");
    app.saved_tmux_bindings.clear();
    app.saved_tmux_status = None;
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
        .map(|line| line.to_string())
}

fn restore_binding_cmd(saved_binding: Option<&str>, key: &str) -> String {
    saved_binding
        .map(|line| format!("tmux {}", line))
        .unwrap_or_else(|| format!("tmux unbind-key -T root {}", key))
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn shell_log_cmd(message: &str) -> String {
    let log_path = crate::paths::log_path().to_string_lossy().to_string();
    format!(
        "printf '%s\\n' {} >> {}",
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

fn tmux_status_value() -> String {
    std::process::Command::new("tmux")
        .args(["show", "-v", "status"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn apply_desired_status(desired_status: &str, current_status: &str) -> Option<String> {
    if current_status.is_empty() {
        return None;
    }

    match desired_status {
        "show" if current_status != "on" => {
            let _ = std::process::Command::new("tmux").args(["set", "status", "on"]).output();
            Some(current_status.to_string())
        }
        "hide" if current_status != "off" => {
            let _ = std::process::Command::new("tmux").args(["set", "status", "off"]).output();
            Some(current_status.to_string())
        }
        _ => None,
    }
}

fn current_tmux_pane_id() -> Option<String> {
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

fn pad_focus_state() -> Option<(String, String)> {
    let pad_pane_id = std::env::var("TMUX_PANE").ok()?;
    let current_pane_id = current_tmux_pane_id()?;
    Some((pad_pane_id, current_pane_id))
}

pub async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(16);
    let mut last_preview_refresh = Instant::now();
    let mut last_full_redraw = Instant::now();

    // Start tmux control pipe for event-driven refresh
    let mut pipe_rx = crate::pipe::start_control_pipe();
    // Two-level debounce: fast for structural, slow for output
    let mut pipe_fast_pending = false;
    let mut pipe_slow_pending = false;
    let mut last_pipe_fast = Instant::now();
    let mut last_pipe_slow = Instant::now();
    let debounce_fast = Duration::from_millis(100);
    let debounce_slow = Duration::from_millis(500);

    loop {
        if app.refresh_after_attach {
            app.refresh_after_attach = false;
            app.refresh_panels();
            app.preview_pane_id = None;
        }

        app.check_scan_result();
        app.check_preview_result();
        app.check_delayed_scan();
        app.check_provider_test_result();

        // Drain hook events (non-blocking)
        let mut pending_hook_events = Vec::new();
        if let Some(ref mut hook_rx) = app.hook_rx {
            loop {
                match hook_rx.try_recv() {
                    Ok(ev) => pending_hook_events.push(ev),
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                        app.hook_rx = None;
                        break;
                    }
                }
            }
        }
        for ev in pending_hook_events {
            app.apply_hook_event(ev);
        }

        // Drain tmux pipe events (non-blocking)
        loop {
            match pipe_rx.try_recv() {
                Ok(ev) => {
                    match ev {
                        crate::pipe::TmuxEvent::WindowChanged
                        | crate::pipe::TmuxEvent::SessionChanged => {
                            pipe_fast_pending = true;
                            last_pipe_fast = Instant::now();
                        }
                        crate::pipe::TmuxEvent::PaneModeChanged
                        | crate::pipe::TmuxEvent::OutputDetected => {
                            pipe_slow_pending = true;
                            last_pipe_slow = Instant::now();
                        }
                        crate::pipe::TmuxEvent::Disconnected => {
                            log_debug!("pipe: disconnected");
                        }
                    }
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => break,
            }
        }

        // Fire debounced pipe-driven scans
        let should_scan = (pipe_fast_pending && last_pipe_fast.elapsed() >= debounce_fast)
            || (pipe_slow_pending && last_pipe_slow.elapsed() >= debounce_slow);
        if should_scan && !app.scan_in_progress {
            pipe_fast_pending = false;
            pipe_slow_pending = false;
            log_debug!("pipe: triggering scan");
            app.trigger_async_scan();
        }

        if last_preview_refresh.elapsed() >= Duration::from_millis(500) {
            app.check_preview_update();
            last_preview_refresh = Instant::now();
        }

        if app.needs_clear {
            terminal.clear()?;
            app.needs_clear = false;
            app.dirty = true;
        }

        if app.dirty {
            terminal.draw(|f| ui::draw(f, app))?;
            app.dirty = false;
        }

        // Deferred status bar restore removed — status is now managed via
        // snapshot/restore in install_return_bindings and the F12 return command.

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            let ev = event::read()?;

            // If we returned from same-session attach (F12/C-q self-restored the bindings
            // in tmux), refresh only after tmux actually switches the active pane back to pad.
            // This avoids treating the initial FocusLost from switching away as a return signal.
            if app.same_session_attached {
                let focus_state = pad_focus_state();
                if let Some((pad_pane_id, current_pane_id)) = focus_state {
                    if current_pane_id == pad_pane_id {
                        log_debug!(
                            "same_session_attached: pad pane active via {:?}, current_pane={} pad_pane={}, refreshing",
                            ev,
                            current_pane_id,
                            pad_pane_id
                        );
                        app.same_session_attached = false;
                        app.saved_tmux_bindings.clear();
                        app.saved_tmux_status = None;
                        app.refresh_panels();
                        app.preview_pane_id = None;
                        app.needs_clear = true;
                        app.dirty = true;
                    } else if matches!(&ev, Event::FocusLost | Event::FocusGained | Event::Resize(_, _)) {
                        log_debug!(
                            "same_session_attached: waiting return event={:?} current_pane={} pad_pane={}",
                            ev,
                            current_pane_id,
                            pad_pane_id
                        );
                    }
                } else if matches!(&ev, Event::FocusLost | Event::FocusGained | Event::Resize(_, _)) {
                    log_debug!("same_session_attached: waiting return event={:?} focus_state=unknown", ev);
                }
            }

            if let Event::Key(key) = ev {
                if key.kind == KeyEventKind::Press {
                    match app.mode {
                        Mode::Normal => {
                            handle_normal_mode(terminal, app, key.code)?;
                        }
                        Mode::Search => handle_search_mode(app, key.code),
                        Mode::Settings => handle_settings_mode(app, key.code),
                        Mode::ThemeSelector => handle_theme_selector_mode(app, key.code),
                        Mode::LanguageSelector => handle_language_selector_mode(app, key.code),
                        Mode::Tree => handle_tree_mode(app, key.code),
                        Mode::TreeSearch => handle_tree_search_mode(app, key.code),
                        Mode::AgentLauncher => handle_agent_launcher_mode(app, key.code),
                        Mode::DeleteConfirm => handle_delete_confirm_mode(app, key.code),
                        Mode::Help => handle_help_mode(app, key.code),
                        Mode::FuzzyPicker => handle_fuzzy_picker_mode(app, key),
                        Mode::RelaySettings => handle_relay_settings_mode(app, key.code),
                        Mode::FilePreview => handle_file_preview_mode(app, key.code),
                        Mode::AgentStyleSettings => handle_agent_style_mode(app, key.code),
                    }
                }
            }

            if let Event::Resize(_, _) = ev {
                terminal.clear()?;
                app.dirty = true;
            }

            // Handle paste events (for relay editing, search, etc.)
            if let Event::Paste(ref text) = ev {
                if app.relay_editing {
                    app.relay_edit_buffer.push_str(text);
                    app.dirty = true;
                } else if app.mode == Mode::Search {
                    app.search_query.push_str(text);
                    app.dirty = true;
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            if app.panels.iter().any(|panel| matches!(panel.state, crate::model::AgentState::Busy))
                && app.last_busy_animation_tick.elapsed() >= Duration::from_millis(80)
            {
                app.busy_animation_frame = app.busy_animation_frame.wrapping_add(1);
                app.last_busy_animation_tick = Instant::now();
                app.dirty = true;
            }
            // Keep interval polling even when control-mode pipe is active.
            // This preserves scanner-driven status refresh without relying on
            // noisy tmux %output notifications.
            if app.config.auto_refresh
                && app.last_refresh.elapsed()
                    >= std::time::Duration::from_secs(app.config.refresh_interval)
            {
                if !app.scan_in_progress {
                    app.trigger_async_scan();
                }
            }
            // Periodic full redraw to clear any accumulated rendering drift
            if last_full_redraw.elapsed() >= Duration::from_secs(30) {
                terminal.clear()?;
                app.dirty = true;
                last_full_redraw = Instant::now();
            }
            last_tick = Instant::now();
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn handle_normal_mode(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    key: KeyCode,
) -> io::Result<()> {
    log_debug!("normal_mode key={:?} show_tree={} panels={}", key, app.show_tree, app.panels.len());
    match key {
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            app.should_quit = true;
        }
        KeyCode::Char('j') | KeyCode::Down => app.next(),
        KeyCode::Char('k') | KeyCode::Up => app.previous(),
        KeyCode::Char('r') | KeyCode::Char('R') => {
            app.refresh_panels();
            app.dirty = true;
        }
        KeyCode::Char('/') => {
            app.mode = Mode::Search;
            app.is_searching = true;
            app.dirty = true;
        }
        KeyCode::Char('?') => {
            app.mode = Mode::Help;
            app.dirty = true;
        }
        KeyCode::Char('1') => app.jump_to(0),
        KeyCode::Char('2') => app.jump_to(1),
        KeyCode::Char('3') => app.jump_to(2),
        KeyCode::Char('4') => app.jump_to(3),
        KeyCode::Char('5') => app.jump_to(4),
        KeyCode::Char('6') => app.jump_to(5),
        KeyCode::Char('7') => app.jump_to(6),
        KeyCode::Char('8') => app.jump_to(7),
        KeyCode::Char('9') => app.jump_to(8),
        KeyCode::F(1) => {
            app.toggle_settings();
            app.dirty = true;
        }
        KeyCode::Char('t') => {
            app.toggle_tree();
        }
        KeyCode::Char('T') => {
            app.open_tree_in_home();
        }
        KeyCode::Char('d') => {
            if let Some(panel) = app.selected_panel() {
                app.delete_target = Some(panel.clone());
                app.mode = Mode::DeleteConfirm;
                app.dirty = true;
            }
        }
        KeyCode::Char(' ') => {
            if app.show_tree {
                if let Some(ref mut tree) = app.file_tree {
                    tree.toggle();
                }
                app.dirty = true;
            }
        }
        KeyCode::Enter => {
            handle_attach(terminal, app)?;
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            // Open fuzzy picker as in-TUI modal instead of leaving alternate screen
            app.open_fuzzy_picker();
        }
        KeyCode::Char('J') => {
            app.preview_follow_bottom = false;
            app.preview_scroll = app.preview_scroll.saturating_add(3);
            app.dirty = true;
        }
        KeyCode::Char('K') => {
            app.preview_follow_bottom = false;
            app.preview_scroll = app.preview_scroll.saturating_sub(3);
            app.dirty = true;
        }
        KeyCode::PageDown => {
            app.preview_follow_bottom = false;
            app.preview_scroll = app.preview_scroll.saturating_add(10);
            app.dirty = true;
        }
        KeyCode::PageUp => {
            app.preview_follow_bottom = false;
            app.preview_scroll = app.preview_scroll.saturating_sub(10);
            app.dirty = true;
        }
        KeyCode::Home => {
            app.preview_follow_bottom = false;
            app.preview_scroll = 0;
            app.dirty = true;
        }
        KeyCode::End => {
            app.preview_follow_bottom = true;
            app.dirty = true;
        }
        _ => {}
    }
    Ok(())
}

fn handle_attach(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    if let Some(panel) = app.selected_panel() {
        let panel = panel.clone();
        log_debug!("attach: pane_id={} agent={} session={} window={}", panel.pane_id, panel.agent_type, panel.session, panel.window_index);

        // Detect if target pane is in the same tmux session
        let current_session = std::env::var("TMUX_PANE").ok().and_then(|_| {
            std::process::Command::new("tmux")
                .args(["display-message", "-p", "#{session_name}"])
                .output()
                .ok()
        }).and_then(|o| if o.status.success() {
            Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
        } else {
            None
        });

        log_debug!(
            "attach: current_session={} target_session={} current_window={} current_pane={}",
            current_session.as_deref().unwrap_or("-"),
            panel.session,
            current_tmux_window_target().as_deref().unwrap_or("-"),
            current_tmux_pane_id().as_deref().unwrap_or("-")
        );

        if current_session.as_deref() == Some(&panel.session) {
            // Same session: install F12/C-q return bindings (zoom + status bar snapshot/restore)
            let target_window = format!("{}:{}", panel.session, panel.window_index);
            log_debug!(
                "attach.same_session: start target_window={} target_pane={} current_window={} current_pane={} target_snapshot={}",
                target_window,
                panel.pane_id,
                current_tmux_window_target().as_deref().unwrap_or("-"),
                current_tmux_pane_id().as_deref().unwrap_or("-"),
                tmux_target_snapshot(&panel.pane_id).as_deref().unwrap_or("-")
            );
            let should_zoom = install_return_bindings(app, &panel.pane_id, &panel.session);
            app.same_session_attached = true;
            log_debug!("attach.same_session: same_session_attached=true should_zoom={}", should_zoom);
            let _ = run_tmux_logged(
                "attach.same_session.select_window",
                vec!["select-window".to_string(), "-t".to_string(), target_window.clone()],
            );
            log_debug!(
                "attach.same_session: after select-window current_window={} current_pane={} target_snapshot={}",
                current_tmux_window_target().as_deref().unwrap_or("-"),
                current_tmux_pane_id().as_deref().unwrap_or("-"),
                tmux_target_snapshot(&panel.pane_id).as_deref().unwrap_or("-")
            );
            let _ = run_tmux_logged(
                "attach.same_session.select_pane",
                vec!["select-pane".to_string(), "-t".to_string(), panel.pane_id.clone()],
            );
            log_debug!(
                "attach.same_session: after select-pane current_window={} current_pane={} target_snapshot={}",
                current_tmux_window_target().as_deref().unwrap_or("-"),
                current_tmux_pane_id().as_deref().unwrap_or("-"),
                tmux_target_snapshot(&panel.pane_id).as_deref().unwrap_or("-")
            );
            // Zoom AFTER select-pane so the user sees the resize happen in-place
            if should_zoom {
                let _ = run_tmux_logged(
                    "attach.same_session.resize_zoom",
                    vec![
                        "resize-pane".to_string(),
                        "-Z".to_string(),
                        "-t".to_string(),
                        panel.pane_id.clone(),
                    ],
                );
                log_debug!(
                    "attach.same_session: after resize-pane current_window={} current_pane={} target_snapshot={}",
                    current_tmux_window_target().as_deref().unwrap_or("-"),
                    current_tmux_pane_id().as_deref().unwrap_or("-"),
                    tmux_target_snapshot(&panel.pane_id).as_deref().unwrap_or("-")
                );
            }
            log_debug!(
                "attach.same_session: handoff complete target_window={} target_pane={} target_snapshot={}",
                target_window,
                panel.pane_id,
                tmux_target_snapshot(&panel.pane_id).as_deref().unwrap_or("-")
            );
            app.dirty = true;
            return Ok(());
        }

        // Cross-session: ensure no stale same-session bindings remain
        if !app.saved_tmux_bindings.is_empty() || app.same_session_attached {
            restore_tmux_bindings(app);
            app.same_session_attached = false;
        }

        // Cross-session: use PTY attach
        // Respect desired status for the attach and restore afterwards.
        let status_before = tmux_status_value();
        let desired_status = app.config.desired_agent_style.status.as_str();
        let status_restore_value = apply_desired_status(desired_status, &status_before);
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

        match attach_to_pane_pty(&panel) {
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

        // Restore status bar to its original state
        if let Some(status) = status_restore_value.as_deref() {
            let _ = std::process::Command::new("tmux")
                .args(["set", "status", status])
                .output();
        }

        app.refresh_after_attach = true;
        app.dirty = true;
    } else {
        log_debug!("attach: no panel selected");
    }
    Ok(())
}

fn handle_fuzzy_picker_mode(app: &mut App, key: crossterm::event::KeyEvent) {
    if let Some(ref mut picker) = app.fuzzy_picker {
        match picker.handle_input(key) {
            None => {
                // No action, continue
                app.dirty = true;
            }
            Some(None) => {
                // Esc — cancelled
                app.close_fuzzy_picker();
            }
            Some(Some(path)) => {
                // Directory selected — clear picker, open agent launcher
                app.fuzzy_picker = None;
                app.open_agent_launcher(std::path::PathBuf::from(path));
                // Keep fuzzy_from_normal = true so agent launcher knows the flow
            }
        }
    }
}

fn handle_relay_settings_mode(app: &mut App, key: KeyCode) {
    use crate::app::state::RelayView;

    if app.relay_editing {
        match key {
            KeyCode::Esc => {
                app.relay_editing = false;
                app.relay_edit_buffer.clear();
                app.dirty = true;
            }
            KeyCode::Enter => {
                let agent_idx = app.relay_selected_agent;
                let prov_idx = app.relay_selected_provider;
                let field = app.relay_edit_field;
                let value = app.relay_edit_buffer.clone();
                if let Some(agent) = app.config.agents.get_mut(agent_idx) {
                    if let Some(prov) = agent.providers.get_mut(prov_idx) {
                        match field {
                            0 => prov.label = value,
                            1 => prov.base_url = value,
                            2 => prov.api_key = value,
                            _ => {}
                        }
                    }
                }
                app.config.save();
                app.relay_editing = false;
                app.relay_edit_buffer.clear();
                app.dirty = true;
            }
            KeyCode::Char(c) => {
                app.relay_edit_buffer.push(c);
                app.dirty = true;
            }
            KeyCode::Backspace => {
                app.relay_edit_buffer.pop();
                app.dirty = true;
            }
            _ => {}
        }
        return;
    }

    match app.relay_view {
        RelayView::AgentList => match key {
            KeyCode::Esc => {
                app.mode = Mode::Settings;
                app.dirty = true;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let max = app.config.agents.len().saturating_sub(1);
                if app.relay_selected_agent < max {
                    app.relay_selected_agent += 1;
                }
                app.dirty = true;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if app.relay_selected_agent > 0 {
                    app.relay_selected_agent -= 1;
                }
                app.dirty = true;
            }
            KeyCode::Enter => {
                app.relay_view = RelayView::ProviderList;
                app.relay_selected_provider = 0;
                app.dirty = true;
            }
            _ => {}
        },
        RelayView::ProviderList => match key {
            KeyCode::Esc => {
                app.relay_view = RelayView::AgentList;
                app.dirty = true;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(agent) = app.config.agents.get(app.relay_selected_agent) {
                    let max = agent.providers.len().saturating_sub(1);
                    if app.relay_selected_provider < max {
                        app.relay_selected_provider += 1;
                    }
                }
                app.dirty = true;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if app.relay_selected_provider > 0 {
                    app.relay_selected_provider -= 1;
                }
                app.dirty = true;
            }
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter => {
                // Enter detail pane for field editing
                if let Some(agent) = app.config.agents.get(app.relay_selected_agent) {
                    if !agent.providers.is_empty() {
                        app.relay_view = RelayView::DetailPane;
                        app.relay_edit_field = 0;
                    }
                }
                app.dirty = true;
            }
            KeyCode::Char(' ') => {
                let agent_idx = app.relay_selected_agent;
                let prov_idx = app.relay_selected_provider;
                if let Some(agent) = app.config.agents.get_mut(agent_idx) {
                    if prov_idx < agent.providers.len() {
                        if agent.active_provider == Some(prov_idx) {
                            agent.active_provider = None;
                        } else {
                            agent.active_provider = Some(prov_idx);
                        }
                        app.config.save();
                        relay::apply_relay_configs(&app.config.agents);
                    }
                }
                app.dirty = true;
            }
            KeyCode::Char('a') => {
                use crate::theme::ProviderConfig;
                if let Some(agent) = app.config.agents.get_mut(app.relay_selected_agent) {
                    agent.providers.push(ProviderConfig {
                        label: format!("provider-{}", agent.providers.len() + 1),
                        base_url: String::new(),
                        api_key: String::new(),
                        test_status: None,
                        test_result: None,
                    });
                    app.relay_selected_provider = agent.providers.len() - 1;
                    app.config.save();
                }
                app.dirty = true;
            }
            KeyCode::Char('d') => {
                let agent_idx = app.relay_selected_agent;
                let prov_idx = app.relay_selected_provider;
                if let Some(agent) = app.config.agents.get_mut(agent_idx) {
                    if prov_idx < agent.providers.len() {
                        agent.providers.remove(prov_idx);
                        match agent.active_provider {
                            Some(i) if i == prov_idx => agent.active_provider = None,
                            Some(i) if i > prov_idx => agent.active_provider = Some(i - 1),
                            _ => {}
                        }
                        if app.relay_selected_provider > 0 && app.relay_selected_provider >= agent.providers.len() {
                            app.relay_selected_provider = agent.providers.len().saturating_sub(1);
                        }
                        app.config.save();
                    }
                }
                app.dirty = true;
            }
            _ => {}
        },
        RelayView::DetailPane => match key {
            KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
                app.relay_view = RelayView::ProviderList;
                app.dirty = true;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                app.relay_edit_field = (app.relay_edit_field + 1) % 3;
                app.dirty = true;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.relay_edit_field = (app.relay_edit_field + 2) % 3;
                app.dirty = true;
            }
            KeyCode::Enter => {
                if let Some(agent) = app.config.agents.get(app.relay_selected_agent) {
                    if let Some(prov) = agent.providers.get(app.relay_selected_provider) {
                        app.relay_edit_buffer = match app.relay_edit_field {
                            0 => prov.label.clone(),
                            1 => prov.base_url.clone(),
                            2 => prov.api_key.clone(),
                            _ => String::new(),
                        };
                        app.relay_editing = true;
                    }
                }
                app.dirty = true;
            }
            KeyCode::Char(' ') => {
                // Test provider connectivity
                app.trigger_provider_test(app.relay_selected_agent, app.relay_selected_provider);
                app.dirty = true;
            }
            _ => {}
        },
    }
}

fn handle_search_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.is_searching = false;
            app.search_query.clear();
            app.dirty = true;
        }
        KeyCode::Enter => {
            app.mode = Mode::Normal;
            app.dirty = true;
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.preview_pane_id = None;
            app.dirty = true;
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.preview_pane_id = None;
            app.dirty = true;
        }
        _ => {}
    }
}

fn handle_settings_mode(app: &mut App, key: KeyCode) {
    if app.settings_searching {
        match key {
            KeyCode::Esc => {
                app.settings_searching = false;
                app.settings_search.clear();
                app.dirty = true;
            }
            KeyCode::Enter => {
                app.settings_searching = false;
                app.dirty = true;
            }
            KeyCode::Char(c) => {
                app.settings_search.push(c);
                app.settings_selected = 0;
                app.dirty = true;
            }
            KeyCode::Backspace => {
                app.settings_search.pop();
                app.settings_selected = 0;
                app.dirty = true;
            }
            _ => {}
        }
        return;
    }

    match key {
        KeyCode::Esc | KeyCode::F(1) => {
            app.settings_open = false;
            app.mode = Mode::Normal;
            app.settings_search.clear();
            app.settings_searching = false;
            app.dirty = true;
        }
        KeyCode::Char('/') => {
            app.settings_searching = true;
            app.settings_search.clear();
            app.dirty = true;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let max = app.filtered_settings_items().len().saturating_sub(1);
            if app.settings_selected < max {
                app.settings_selected += 1;
            }
            app.dirty = true;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.settings_selected > 0 {
                app.settings_selected -= 1;
            }
            app.dirty = true;
        }
        KeyCode::Char('1') => {
            app.settings_selected = 0;
            app.dirty = true;
        }
        KeyCode::Char('2') => {
            app.settings_selected = 1.min(app.filtered_settings_items().len().saturating_sub(1));
            app.dirty = true;
        }
        KeyCode::Char('3') => {
            app.settings_selected = 2.min(app.filtered_settings_items().len().saturating_sub(1));
            app.dirty = true;
        }
        KeyCode::Char('4') => {
            app.settings_selected = 3.min(app.filtered_settings_items().len().saturating_sub(1));
            app.dirty = true;
        }
        KeyCode::Enter => {
            let items = app.filtered_settings_items();
            if let Some((id, _, _, _, editable)) = items.get(app.settings_selected) {
                if *editable {
                    match *id {
                        "theme" => app.open_theme_selector(),
                        "auto_refresh" => {
                            app.config.auto_refresh = !app.config.auto_refresh;
                            app.config.save();
                        }
                        "relay" => {
                            app.relay_selected_agent = 0;
                            app.relay_selected_provider = 0;
                            app.relay_edit_field = 0;
                            app.relay_editing = false;
                            app.relay_edit_buffer.clear();
                            app.relay_view = crate::app::state::RelayView::AgentList;
                            app.mode = Mode::RelaySettings;
                        }
                        "agent_style" => {
                            app.agent_style_selected = 0;
                            app.mode = Mode::AgentStyleSettings;
                        }
                        "language" => {
                            app.open_language_selector();
                        }
                        _ => {}
                    }
                }
            }
            app.dirty = true;
        }
        _ => {}
    }
}

fn handle_theme_selector_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.close_theme_selector();
            app.dirty = true;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let max = App::available_themes().len().saturating_sub(1);
            if app.theme_selected < max {
                app.theme_selected += 1;
            }
            let themes = App::available_themes();
            if let Some((name, _)) = themes.get(app.theme_selected) {
                app.preview_theme(name);
            }
            app.dirty = true;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.theme_selected > 0 {
                app.theme_selected -= 1;
            }
            let themes = App::available_themes();
            if let Some((name, _)) = themes.get(app.theme_selected) {
                app.preview_theme(name);
            }
            app.dirty = true;
        }
        KeyCode::Char(c @ '1'..='9') => {
            let idx = (c as usize) - ('1' as usize);
            app.theme_selected = idx.min(App::available_themes().len().saturating_sub(1));
            let themes = App::available_themes();
            if let Some((name, _)) = themes.get(app.theme_selected) {
                app.preview_theme(name);
            }
            app.dirty = true;
        }
        KeyCode::Enter => {
            let themes = App::available_themes();
            if let Some((name, _)) = themes.get(app.theme_selected) {
                app.apply_theme(name);
                app.theme_selector_open = false;
                app.mode = crate::app::state::Mode::Settings;
            }
            app.dirty = true;
        }
        _ => {}
    }
}

fn handle_language_selector_mode(app: &mut App, key: KeyCode) {
    let locales = App::available_locales();
    match key {
        KeyCode::Esc => {
            app.close_language_selector();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let max = locales.len().saturating_sub(1);
            if app.language_selected < max {
                app.language_selected += 1;
            }
            // Hot-reload preview
            if let Some(l) = locales.get(app.language_selected) {
                app.locale = *l;
            }
            app.dirty = true;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.language_selected > 0 {
                app.language_selected -= 1;
            }
            if let Some(l) = locales.get(app.language_selected) {
                app.locale = *l;
            }
            app.dirty = true;
        }
        KeyCode::Enter => {
            if let Some(l) = locales.get(app.language_selected) {
                app.locale = *l;
                app.config.language = l.as_str().to_string();
                app.config.save();
            }
            app.mode = crate::app::state::Mode::Settings;
            app.dirty = true;
        }
        _ => {}
    }
}

fn handle_tree_mode(app: &mut App, key: KeyCode) {
    if let Some(ref mut tree) = app.file_tree {
        log_debug!("tree_mode key={:?} path={} selected={:?}", key, tree.current_path.display(), tree.state.selected());
        match key {
            KeyCode::Esc => {
                app.close_tree();
                app.dirty = true;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                tree.next();
                app.update_file_preview();
                app.dirty = true;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                tree.previous();
                app.update_file_preview();
                app.dirty = true;
            }
            KeyCode::Char(' ') => {
                tree.toggle();
                app.update_file_preview();
                app.dirty = true;
            }
            KeyCode::Enter => {
                let entry_name = tree.selected().map(|e| e.name.clone()).unwrap_or_default();
                log_debug!("tree_mode enter: entry={}", entry_name);
                let selected_is_dir = tree.selected().map(|e| e.is_dir).unwrap_or(false);
                if selected_is_dir {
                    tree.enter();
                    app.update_file_preview();
                } else {
                    app.mode = Mode::FilePreview;
                    app.file_preview_scroll = 0;
                }
                app.dirty = true;
            }
            KeyCode::Backspace => {
                tree.go_up();
                app.update_file_preview();
                app.dirty = true;
            }
            KeyCode::Char('/') => {
                app.mode = Mode::TreeSearch;
                tree.start_search();
                app.dirty = true;
            }
            KeyCode::Char('c') => {
                let target_path = tree
                    .selected()
                    .filter(|e| e.is_dir)
                    .map(|e| e.path.clone());
                if let Some(path) = target_path {
                    log_debug!("tree_mode: open agent launcher at {}", path.display());
                    app.open_agent_launcher(path);
                }
            }
            KeyCode::Char('T') => {
                app.open_tree_in_home();
            }
            KeyCode::Char('t') => {
                app.toggle_tree();
            }
            KeyCode::Char('J') => {
                app.file_preview_scroll = app.file_preview_scroll.saturating_add(3);
                app.dirty = true;
            }
            KeyCode::Char('K') => {
                app.file_preview_scroll = app.file_preview_scroll.saturating_sub(3);
                app.dirty = true;
            }
            KeyCode::PageDown => {
                app.file_preview_scroll = app.file_preview_scroll.saturating_add(10);
                app.dirty = true;
            }
            KeyCode::PageUp => {
                app.file_preview_scroll = app.file_preview_scroll.saturating_sub(10);
                app.dirty = true;
            }
            _ => {}
        }
    }
}

fn handle_file_preview_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = Mode::Tree;
            app.dirty = true;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.file_preview_scroll = app.file_preview_scroll.saturating_add(1);
            app.dirty = true;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.file_preview_scroll = app.file_preview_scroll.saturating_sub(1);
            app.dirty = true;
        }
        KeyCode::Char('J') => {
            app.file_preview_scroll = app.file_preview_scroll.saturating_add(3);
            app.dirty = true;
        }
        KeyCode::Char('K') => {
            app.file_preview_scroll = app.file_preview_scroll.saturating_sub(3);
            app.dirty = true;
        }
        KeyCode::PageDown => {
            app.file_preview_scroll = app.file_preview_scroll.saturating_add(20);
            app.dirty = true;
        }
        KeyCode::PageUp => {
            app.file_preview_scroll = app.file_preview_scroll.saturating_sub(20);
            app.dirty = true;
        }
        KeyCode::Home => {
            app.file_preview_scroll = 0;
            app.dirty = true;
        }
        KeyCode::End => {
            app.file_preview_scroll = u16::MAX;
            app.dirty = true;
        }
        _ => {}
    }
}

fn handle_tree_search_mode(app: &mut App, key: KeyCode) {
    if let Some(ref mut tree) = app.file_tree {
        match key {
            KeyCode::Esc => {
                tree.cancel_search();
                app.mode = Mode::Tree;
                app.update_file_preview();
                app.dirty = true;
            }
            KeyCode::Enter => {
                tree.cancel_search();
                app.mode = Mode::Tree;
                app.update_file_preview();
                app.dirty = true;
            }
            KeyCode::Char(c) => {
                tree.search_input(c);
                app.update_file_preview();
                app.dirty = true;
            }
            KeyCode::Backspace => {
                tree.search_backspace();
                app.update_file_preview();
                app.dirty = true;
            }
            _ => {}
        }
    }
}

fn handle_agent_launcher_mode(app: &mut App, key: KeyCode) {
    // Capture whether this launch came from the fuzzy picker (Normal mode 'c' flow)
    let from_fuzzy = app.fuzzy_from_normal;

    if let Some(ref mut launcher) = app.agent_launcher {
        log_debug!("agent_launcher key={:?} selected={} from_fuzzy={}", key, launcher.selected, from_fuzzy);
        match key {
            KeyCode::Esc => {
                app.close_agent_launcher();
                app.dirty = true;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                launcher.next();
                app.dirty = true;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                launcher.previous();
                app.dirty = true;
            }
            KeyCode::Enter => {
                if let Some(agent) = launcher.selected_agent() {
                    let target_dir = launcher.target_dir.clone();
                    let agent_cmd = agent.1.to_string();
                    log_debug!("agent_launcher: launching cmd={} dir={}", agent_cmd, target_dir.display());

                    app.close_agent_launcher();
                    app.dirty = true;

                    // Ensure relay config is applied before spawning agent
                    relay::apply_relay_configs(&app.config.agents);

                    if from_fuzzy {
                        // From Normal mode 'c' key: create a new tmux session with agent
                        let dir_str = target_dir.to_string_lossy().to_string();
                        let cmd = agent_cmd.clone();
                        log_debug!("agent_launcher: from_fuzzy=true, spawning create_session_with_agent dir={} cmd={}", dir_str, cmd);
                        std::thread::spawn(move || {
                            match session::create_session_with_agent(&dir_str, &cmd) {
                                Ok(()) => log_debug!("agent_launcher: create_session_with_agent 成功"),
                                Err(e) => log_debug!("agent_launcher: create_session_with_agent 失败: {}", e),
                            }
                        });
                    } else {
                        // From Tree mode: open new window in current session
                        std::thread::spawn(move || {
                            let _ = std::process::Command::new("tmux")
                                .args(["new-window", "-c", &target_dir.to_string_lossy()])
                                .arg(&agent_cmd)
                                .spawn();
                        });
                    }

                    // Schedule a delayed scan so the new session/window has time to start
                    app.schedule_delayed_scan(800);
                }
            }
            _ => {}
        }
    }
}

fn handle_delete_confirm_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let Some(panel) = app.delete_target.take() {
                app.delete_panel(&panel);
            }
            app.mode = Mode::Normal;
            app.dirty = true;
        }
        _ => {
            app.delete_target = None;
            app.mode = Mode::Normal;
            app.dirty = true;
        }
    }
}

fn handle_help_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
            app.mode = Mode::Normal;
            app.dirty = true;
        }
        _ => {}
    }
}

fn handle_agent_style_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.mode = Mode::Settings;
            app.dirty = true;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if app.agent_style_selected < 1 {
                app.agent_style_selected += 1;
            }
            app.dirty = true;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.agent_style_selected > 0 {
                app.agent_style_selected -= 1;
            }
            app.dirty = true;
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            match app.agent_style_selected {
                0 => {
                    // Toggle zoom: auto -> keep -> auto
                    app.config.desired_agent_style.zoom = if app.config.desired_agent_style.zoom == "auto" {
                        "keep".to_string()
                    } else {
                        "auto".to_string()
                    };
                }
                1 => {
                    // Cycle status: show -> hide -> keep -> show
                    app.config.desired_agent_style.status = match app.config.desired_agent_style.status.as_str() {
                        "show" => "hide".to_string(),
                        "hide" => "keep".to_string(),
                        _ => "show".to_string(),
                    };
                }
                _ => {}
            }
            app.config.save();
            app.dirty = true;
        }
        _ => {}
    }
}
