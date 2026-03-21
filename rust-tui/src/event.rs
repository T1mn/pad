use crate::app::App;
use crate::app::state::Mode;
use crate::log_debug;
use crate::pty::attach_to_pane_pty;
use crate::session::create_new_session_fuzzy;
use crate::ui;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Write};
use std::time::{Duration, Instant};

/// Save current F12/C-q root bindings, then install temporary bindings that switch back to pad's pane.
/// The return binding itself restores the original bindings, so no need for pad to restore them.
fn save_and_install_return_bindings(app: &mut App) {
    let pad_pane_id = match std::env::var("TMUX_PANE") {
        Ok(id) => id,   // e.g. "%280"
        Err(_) => {
            log_debug!("save_and_install_return_bindings: TMUX_PANE not set");
            return;
        }
    };

    // Get pad's session:window target for select-window
    let pad_win_target = std::process::Command::new("tmux")
        .args(["display-message", "-t", &pad_pane_id, "-p", "#{session_name}:#{window_index}"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

    let win_target = match pad_win_target {
        Some(t) => t,
        None => {
            log_debug!("save_and_install_return_bindings: cannot determine pad window target");
            return;
        }
    };

    // Get current root key table bindings for F12 and C-q
    let output = std::process::Command::new("tmux")
        .args(["list-keys", "-T", "root"])
        .output();

    let mut saved_f12: Option<String> = None;
    let mut saved_cq: Option<String> = None;
    if let Ok(out) = output {
        let lines = String::from_utf8_lossy(&out.stdout);
        for line in lines.lines() {
            // Each line looks like: bind-key -T root F12 <cmd...>
            let trimmed = line.trim();
            if trimmed.contains(" F12 ") {
                saved_f12 = Some(trimmed.to_string());
            }
            if trimmed.contains(" C-q ") {
                saved_cq = Some(trimmed.to_string());
            }
        }
    }

    // Build restore commands that the return binding will execute
    let restore_f12 = match &saved_f12 {
        Some(line) => format!("tmux {}", line),
        None => "tmux unbind-key -T root F12".to_string(),
    };
    let restore_cq = match &saved_cq {
        Some(line) => format!("tmux {}", line),
        None => "tmux unbind-key -T root C-q".to_string(),
    };

    // The return command: switch back to pad, then restore both keys
    let return_cmd_f12 = format!(
        "tmux select-window -t '{}' && tmux select-pane -t '{}' && {} && {}",
        win_target, pad_pane_id, restore_f12, restore_cq
    );
    let return_cmd_cq = return_cmd_f12.clone();

    let _ = std::process::Command::new("tmux")
        .args(["bind-key", "-T", "root", "F12", "run-shell", &return_cmd_f12])
        .output();
    let _ = std::process::Command::new("tmux")
        .args(["bind-key", "-T", "root", "C-q", "run-shell", &return_cmd_cq])
        .output();

    // Save info for pad's own cleanup (crash/quit while attached)
    app.saved_tmux_bindings = Vec::new();
    if let Some(line) = saved_f12 { app.saved_tmux_bindings.push(line); }
    if let Some(line) = saved_cq { app.saved_tmux_bindings.push(line); }

    log_debug!("installed self-restoring return bindings: F12/C-q -> select-window + restore");
}

/// Restore original tmux bindings — only used as safety net on pad quit/crash,
/// since normal return via F12/C-q self-restores.
pub fn restore_tmux_bindings(app: &mut App) {
    let mut restored_f12 = false;
    let mut restored_cq = false;

    for line in &app.saved_tmux_bindings {
        let _ = std::process::Command::new("tmux")
            .args(line.split_whitespace().collect::<Vec<&str>>())
            .output();
        if line.contains(" F12 ") {
            restored_f12 = true;
        }
        if line.contains(" C-q ") {
            restored_cq = true;
        }
    }

    if !restored_f12 {
        let _ = std::process::Command::new("tmux")
            .args(["unbind-key", "-T", "root", "F12"])
            .output();
    }
    if !restored_cq {
        let _ = std::process::Command::new("tmux")
            .args(["unbind-key", "-T", "root", "C-q"])
            .output();
    }

    log_debug!("restore_tmux_bindings: cleaned up (f12={}, cq={})", restored_f12, restored_cq);
    app.saved_tmux_bindings.clear();
}

pub async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(16);
    let mut last_preview_refresh = Instant::now();

    loop {
        if app.refresh_after_attach {
            app.refresh_after_attach = false;
            app.refresh_panels();
            app.preview_pane_id = None;
        }

        app.check_scan_result();
        app.check_preview_result();

        if last_preview_refresh.elapsed() >= Duration::from_millis(500) {
            app.check_preview_update();
            last_preview_refresh = Instant::now();
        }

        if app.dirty {
            terminal.draw(|f| ui::draw(f, app))?;
            app.dirty = false;
        }

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            let ev = event::read()?;

            // If we returned from same-session attach (F12/C-q self-restored the bindings
            // in tmux), detect that pad has focus again and refresh
            if app.same_session_attached {
                match &ev {
                    Event::FocusGained | Event::Key(_) => {
                        log_debug!("same_session_attached: pad regained focus, refreshing");
                        app.same_session_attached = false;
                        app.saved_tmux_bindings.clear(); // bindings already self-restored by tmux
                        app.refresh_panels();
                        app.preview_pane_id = None;
                        app.dirty = true;
                    }
                    _ => {}
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
                        Mode::Tree => handle_tree_mode(app, key.code),
                        Mode::TreeSearch => handle_tree_search_mode(app, key.code),
                        Mode::AgentLauncher => handle_agent_launcher_mode(app, key.code),
                        Mode::DeleteConfirm => handle_delete_confirm_mode(app, key.code),
                        Mode::Help => handle_help_mode(app, key.code),
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            if app.config.auto_refresh
                && app.last_refresh.elapsed()
                    >= std::time::Duration::from_secs(app.config.refresh_interval)
            {
                if !app.scan_in_progress {
                    app.trigger_async_scan();
                }
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
            handle_create_session(terminal, app)?;
        }
        KeyCode::PageDown => {
            app.preview_scroll = app.preview_scroll.saturating_add(10);
            app.dirty = true;
        }
        KeyCode::PageUp => {
            app.preview_scroll = app.preview_scroll.saturating_sub(10);
            app.dirty = true;
        }
        KeyCode::Home => {
            app.preview_scroll = 0;
            app.dirty = true;
        }
        KeyCode::End => {
            app.preview_scroll = u16::MAX;
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

        if current_session.as_deref() == Some(&panel.session) {
            // Same session: install self-restoring F12/Ctrl+Q bindings, then switch
            log_debug!("attach: same session '{}', using select-window/select-pane", panel.session);
            save_and_install_return_bindings(app);
            app.same_session_attached = true;
            let _ = std::process::Command::new("tmux")
                .args(["select-window", "-t", &format!("{}:{}", panel.session, panel.window_index)])
                .output();
            let _ = std::process::Command::new("tmux")
                .args(["select-pane", "-t", &panel.pane_id])
                .output();
            app.dirty = true;
            return Ok(());
        }

        // Cross-session: ensure no stale same-session bindings remain
        if !app.saved_tmux_bindings.is_empty() || app.same_session_attached {
            restore_tmux_bindings(app);
            app.same_session_attached = false;
        }

        // Cross-session: use PTY attach
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

        app.refresh_after_attach = true;
        app.dirty = true;
    } else {
        log_debug!("attach: no panel selected");
    }
    Ok(())
}

fn handle_create_session(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    print!("\x1b[2J\x1b[H");
    io::stdout().flush()?;

    if let Err(e) = create_new_session_fuzzy() {
        println!("Error: {}", e);
        println!("\nPress any key to continue...");
        let _ = crossterm::event::read();
    }

    print!("\x1b[2J\x1b[H");
    io::stdout().flush()?;

    enable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture
    )?;

    app.refresh_panels();
    Ok(())
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
    match key {
        KeyCode::Esc | KeyCode::F(1) => {
            app.settings_open = false;
            app.mode = Mode::Normal;
            app.dirty = true;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let max = app.settings_items().len().saturating_sub(1);
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
            app.settings_selected = 1.min(app.settings_items().len().saturating_sub(1));
            app.dirty = true;
        }
        KeyCode::Char('3') => {
            app.settings_selected = 2.min(app.settings_items().len().saturating_sub(1));
            app.dirty = true;
        }
        KeyCode::Char('4') => {
            app.settings_selected = 3.min(app.settings_items().len().saturating_sub(1));
            app.dirty = true;
        }
        KeyCode::Enter => {
            let items = app.settings_items();
            if let Some((name, _, _, editable)) = items.get(app.settings_selected) {
                if *editable {
                    match *name {
                        "Theme" => app.open_theme_selector(),
                        "Auto Refresh" => {
                            app.config.auto_refresh = !app.config.auto_refresh;
                            app.config.save();
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
                tree.enter();
                app.update_file_preview();
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
    if let Some(ref mut launcher) = app.agent_launcher {
        log_debug!("agent_launcher key={:?} selected={}", key, launcher.selected);
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

                    std::thread::spawn(move || {
                        let _ = std::process::Command::new("tmux")
                            .args(["new-window", "-c", &target_dir.to_string_lossy()])
                            .arg(&agent_cmd)
                            .spawn();
                    });
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
