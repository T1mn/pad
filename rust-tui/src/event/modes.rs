use crate::app::state::Mode;
use crate::app::App;
use crate::log_debug;
use crate::relay;
use crate::session;
use crate::telegram;
use crossterm::event::{KeyCode, KeyEvent};

pub(super) fn handle_fuzzy_picker_mode(app: &mut App, key: KeyEvent) {
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

pub(super) fn handle_relay_settings_mode(app: &mut App, key: KeyCode) {
    use crate::app::state::RelayView;

    let relay_field_count = |app: &App| -> usize {
        match app
            .config
            .agents
            .get(app.relay_selected_agent)
            .map(|agent| agent.name.as_str())
        {
            Some("codex") => 3,
            _ => 3,
        }
    };

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
                            2 => {
                                prov.api_key = value;
                                if agent.name == "codex" {
                                    prov.env_key.clear();
                                }
                            }
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
                        env_key: String::new(),
                        wire_api: "responses".to_string(),
                        test_status: None,
                        test_http_status: None,
                        test_latency_ms: None,
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
                        if app.relay_selected_provider > 0
                            && app.relay_selected_provider >= agent.providers.len()
                        {
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
                app.relay_edit_field = (app.relay_edit_field + 1) % relay_field_count(app);
                app.dirty = true;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let count = relay_field_count(app);
                app.relay_edit_field = (app.relay_edit_field + count - 1) % count;
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

pub(super) fn handle_search_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.is_searching = false;
            app.search_query.clear();
            app.invalidate_sidebar_visible_cache();
            app.sync_sidebar_selection();
            app.dirty = true;
        }
        KeyCode::Enter => {
            app.mode = Mode::Normal;
            app.sync_sidebar_selection();
            app.dirty = true;
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.invalidate_sidebar_visible_cache();
            app.sync_sidebar_selection();
            app.invalidate_preview();
            app.dirty = true;
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.invalidate_sidebar_visible_cache();
            app.sync_sidebar_selection();
            app.invalidate_preview();
            app.dirty = true;
        }
        _ => {}
    }
}

pub(super) fn handle_settings_mode(app: &mut App, key: KeyCode) {
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
        KeyCode::Char('5') => {
            app.settings_selected = 4.min(app.filtered_settings_items().len().saturating_sub(1));
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
                        "telegram" => {
                            app.telegram_selected_field = 0;
                            app.telegram_editing = false;
                            app.telegram_edit_buffer.clear();
                            app.mode = Mode::TelegramSettings;
                        }
                        "agent_style" => {
                            app.agent_style_selected = 0;
                            app.mode = Mode::AgentStyleSettings;
                        }
                        "preview_mode" => {
                            app.config.preview.mode = match app.config.preview.mode.as_str() {
                                "auto" => "tmux".to_string(),
                                "tmux" => "session".to_string(),
                                _ => "auto".to_string(),
                            };
                            app.config.save();
                            app.invalidate_preview();
                        }
                        "display_mode" => {
                            let next_scope = if app.config.display.session_scope == "live" {
                                "all"
                            } else {
                                "live"
                            };
                            app.apply_display_session_scope(next_scope, true);
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

pub(super) fn handle_thread_action_confirm_mode(app: &mut App, key: KeyCode) {
    if app.thread_meta_editing {
        match key {
            KeyCode::Esc => {
                app.cancel_thread_meta_edit();
            }
            KeyCode::Enter => {
                app.commit_thread_meta_edit();
            }
            KeyCode::Backspace => {
                app.thread_meta_buffer.pop();
                app.dirty = true;
            }
            KeyCode::Char(c) => {
                app.thread_meta_buffer.push(c);
                app.dirty = true;
            }
            _ => {}
        }
        return;
    }

    match key {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.confirm_thread_action();
        }
        _ => {
            app.close_thread_action_confirm();
        }
    }
}

pub(super) fn handle_theme_selector_mode(app: &mut App, key: KeyCode) {
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

pub(super) fn handle_language_selector_mode(app: &mut App, key: KeyCode) {
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

pub(super) fn handle_tree_mode(app: &mut App, key: KeyCode) {
    if let Some(ref mut tree) = app.file_tree {
        log_debug!(
            "tree_mode key={:?} path={} selected={:?}",
            key,
            tree.current_path.display(),
            tree.state.selected()
        );
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
                let target_path = tree.selected().filter(|e| e.is_dir).map(|e| e.path.clone());
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

pub(super) fn handle_file_preview_mode(app: &mut App, key: KeyCode) {
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

pub(super) fn handle_tree_search_mode(app: &mut App, key: KeyCode) {
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

pub(super) fn handle_agent_launcher_mode(app: &mut App, key: KeyCode) {
    // Capture whether this launch came from the fuzzy picker (Normal mode 'c' flow)
    let from_fuzzy = app.fuzzy_from_normal;

    if let Some(ref mut launcher) = app.agent_launcher {
        log_debug!(
            "agent_launcher key={:?} selected={} from_fuzzy={}",
            key,
            launcher.selected,
            from_fuzzy
        );
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
                    log_debug!(
                        "agent_launcher: launching cmd={} dir={}",
                        agent_cmd,
                        target_dir.display()
                    );

                    app.close_agent_launcher();
                    app.dirty = true;

                    // Ensure relay config is applied before spawning agent
                    relay::apply_relay_configs(&app.config.agents);

                    if from_fuzzy {
                        // From Normal mode 'c' key: create a new tmux session with agent
                        let dir_str = target_dir.to_string_lossy().to_string();
                        let cmd = agent_cmd.clone();
                        if !app.saved_tmux_bindings.is_empty() || app.same_session_attached {
                            super::restore_tmux_bindings(app);
                            app.same_session_attached = false;
                        }
                        log_debug!(
                            "agent_launcher: from_fuzzy=true, create_session_with_agent dir={} cmd={}",
                            dir_str,
                            cmd
                        );
                        match session::create_session_with_agent(app, &dir_str, &cmd) {
                            Ok(()) => log_debug!("agent_launcher: create_session_with_agent 成功"),
                            Err(e) => {
                                log_debug!("agent_launcher: create_session_with_agent 失败: {}", e)
                            }
                        }
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

pub(super) fn handle_delete_confirm_mode(app: &mut App, key: KeyCode) {
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

pub(super) fn handle_help_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
            app.mode = Mode::Normal;
            app.dirty = true;
        }
        _ => {}
    }
}

pub(super) fn handle_agent_style_mode(app: &mut App, key: KeyCode) {
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
                    app.config.desired_agent_style.zoom =
                        if app.config.desired_agent_style.zoom == "auto" {
                            "keep".to_string()
                        } else {
                            "auto".to_string()
                        };
                }
                1 => {
                    // Cycle status: show -> hide -> keep -> show
                    app.config.desired_agent_style.status =
                        match app.config.desired_agent_style.status.as_str() {
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

pub(super) fn handle_telegram_settings_mode(app: &mut App, key: KeyCode) {
    if app.telegram_editing {
        match key {
            KeyCode::Esc => {
                app.telegram_editing = false;
                app.telegram_edit_buffer.clear();
                app.dirty = true;
            }
            KeyCode::Enter => {
                let mut restart_needed = false;
                match app.telegram_selected_field {
                    1 => {
                        restart_needed = app.config.telegram.bot_token != app.telegram_edit_buffer;
                        app.config.telegram.bot_token = app.telegram_edit_buffer.clone();
                    }
                    2 => app.config.telegram.chat_id = app.telegram_edit_buffer.clone(),
                    _ => {}
                }
                app.config.save();
                let daemon_result = if restart_needed {
                    telegram::restart_daemon(&app.config)
                } else {
                    telegram::sync_daemon(&app.config)
                };
                if let Err(err) = daemon_result {
                    log_debug!("telegram: daemon sync failed after settings save: {}", err);
                }
                app.telegram_editing = false;
                app.telegram_edit_buffer.clear();
                app.dirty = true;
            }
            KeyCode::Backspace => {
                app.telegram_edit_buffer.pop();
                app.dirty = true;
            }
            KeyCode::Char(c) => {
                app.telegram_edit_buffer.push(c);
                app.dirty = true;
            }
            _ => {}
        }
        return;
    }

    match key {
        KeyCode::Esc => {
            app.mode = Mode::Settings;
            app.dirty = true;
        }
        KeyCode::Char('r') => {
            restart_telegram_daemon(app);
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if app.telegram_selected_field < 3 {
                app.telegram_selected_field += 1;
            }
            app.dirty = true;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.telegram_selected_field > 0 {
                app.telegram_selected_field -= 1;
            }
            app.dirty = true;
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            match app.telegram_selected_field {
                0 => {
                    app.config.telegram.enabled = !app.config.telegram.enabled;
                    app.config.save();
                    if let Err(err) = telegram::sync_daemon(&app.config) {
                        log_debug!("telegram: daemon sync failed after toggle: {}", err);
                    }
                }
                1 => {
                    app.telegram_edit_buffer = app.config.telegram.bot_token.clone();
                    app.telegram_editing = true;
                }
                2 => {
                    app.telegram_edit_buffer = app.config.telegram.chat_id.clone();
                    app.telegram_editing = true;
                }
                3 => {
                    restart_telegram_daemon(app);
                }
                _ => {}
            }
            app.dirty = true;
        }
        _ => {}
    }
}

fn restart_telegram_daemon(app: &mut App) {
    if let Err(err) = telegram::restart_daemon(&app.config) {
        log_debug!("telegram: restart failed from settings: {}", err);
    }
    app.dirty = true;
}
