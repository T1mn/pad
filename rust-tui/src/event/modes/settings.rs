use crate::app::state::{RelayView, SettingsDetailKind, SettingsFocus};
use crate::app::App;
use crate::log_debug;
use crate::{relay, telegram};
use crossterm::event::KeyCode;

pub(crate) fn handle_settings_mode(app: &mut App, key: KeyCode) {
    if app.settings_searching {
        match key {
            KeyCode::Esc => {
                app.settings_searching = false;
                app.settings_search.clear();
                app.dirty = true;
            }
            KeyCode::Enter => {
                app.settings_searching = false;
                if !app.filtered_settings_items().is_empty() {
                    app.enter_settings_detail();
                } else {
                    app.dirty = true;
                }
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

    if app.settings_focus == SettingsFocus::Detail && handle_settings_detail_mode(app, key) {
        return;
    }

    match key {
        KeyCode::Esc | KeyCode::F(1) => {
            app.close_settings();
        }
        KeyCode::Char('/') => {
            app.settings_focus = SettingsFocus::List;
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
        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
            app.enter_settings_detail();
        }
        _ => {}
    }
}

fn handle_settings_detail_mode(app: &mut App, key: KeyCode) -> bool {
    if key == KeyCode::F(1) {
        app.close_settings();
        return true;
    }
    if key == KeyCode::Char('/') {
        app.leave_settings_detail();
        app.settings_searching = true;
        app.settings_search.clear();
        app.dirty = true;
        return true;
    }
    match app.current_settings_detail_kind() {
        Some(SettingsDetailKind::Theme) => handle_theme_detail_mode(app, key),
        Some(SettingsDetailKind::Language) => handle_language_detail_mode(app, key),
        Some(SettingsDetailKind::AgentStyle) => handle_agent_style_detail_mode(app, key),
        Some(SettingsDetailKind::Relay) => handle_relay_detail_mode(app, key),
        Some(SettingsDetailKind::Telegram) => handle_telegram_detail_mode(app, key),
        Some(SettingsDetailKind::AutoRefresh) => handle_auto_refresh_detail_mode(app, key),
        Some(SettingsDetailKind::PreviewMode) => handle_preview_mode_detail_mode(app, key),
        Some(SettingsDetailKind::DisplayMode) => handle_display_mode_detail_mode(app, key),
        Some(SettingsDetailKind::RefreshInterval | SettingsDetailKind::Version) => {
            match key {
                KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => app.leave_settings_detail(),
                _ => {}
            }
            true
        }
        None => false,
    }
}

fn handle_theme_detail_mode(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
            app.leave_settings_detail();
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
        KeyCode::Enter | KeyCode::Char(' ') => {
            let themes = App::available_themes();
            if let Some((name, _)) = themes.get(app.theme_selected) {
                app.apply_theme(name);
                app.settings_focus = SettingsFocus::List;
                app.dirty = true;
            }
        }
        _ => {}
    }
    true
}

fn handle_language_detail_mode(app: &mut App, key: KeyCode) -> bool {
    let locales = App::available_locales();
    match key {
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
            app.leave_settings_detail();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let max = locales.len().saturating_sub(1);
            if app.language_selected < max {
                app.language_selected += 1;
            }
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
        KeyCode::Enter | KeyCode::Char(' ') => {
            if let Some(l) = locales.get(app.language_selected) {
                app.locale = *l;
                app.config.language = l.as_str().to_string();
                app.config.save();
            }
            app.settings_focus = SettingsFocus::List;
            app.dirty = true;
        }
        _ => {}
    }
    true
}

fn handle_agent_style_detail_mode(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
            app.leave_settings_detail();
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
                    app.config.desired_agent_style.zoom =
                        if app.config.desired_agent_style.zoom == "auto" {
                            "keep".to_string()
                        } else {
                            "auto".to_string()
                        };
                }
                1 => {
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
    true
}

fn handle_auto_refresh_detail_mode(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => app.leave_settings_detail(),
        KeyCode::Enter | KeyCode::Char(' ') => {
            app.config.auto_refresh = !app.config.auto_refresh;
            app.config.save();
            app.dirty = true;
        }
        _ => {}
    }
    true
}

fn handle_preview_mode_detail_mode(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => app.leave_settings_detail(),
        KeyCode::Enter | KeyCode::Char(' ') => {
            app.config.preview.mode = match app.config.preview.mode.as_str() {
                "auto" => "tmux".to_string(),
                "tmux" => "session".to_string(),
                _ => "auto".to_string(),
            };
            app.config.save();
            app.invalidate_preview();
            app.dirty = true;
        }
        _ => {}
    }
    true
}

fn handle_display_mode_detail_mode(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => app.leave_settings_detail(),
        KeyCode::Enter | KeyCode::Char(' ') => {
            let next_scope = if app.config.display.session_scope == "live" {
                "all"
            } else {
                "live"
            };
            app.apply_display_session_scope(next_scope, true);
        }
        _ => {}
    }
    true
}

fn handle_relay_detail_mode(app: &mut App, key: KeyCode) -> bool {
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
            KeyCode::Backspace => {
                app.relay_edit_buffer.pop();
                app.dirty = true;
            }
            KeyCode::Char(c) => {
                app.relay_edit_buffer.push(c);
                app.dirty = true;
            }
            _ => {}
        }
        return true;
    }

    match app.relay_view {
        RelayView::AgentList => match key {
            KeyCode::Esc => app.leave_settings_detail(),
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
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                app.relay_view = RelayView::ProviderList;
                let active = app
                    .config
                    .agents
                    .get(app.relay_selected_agent)
                    .and_then(|agent| agent.active_provider)
                    .unwrap_or(0);
                app.relay_selected_provider = active;
                app.dirty = true;
            }
            _ => {}
        },
        RelayView::ProviderList => match key {
            KeyCode::Esc => app.leave_settings_detail(),
            KeyCode::Left | KeyCode::Char('h') => {
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
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                if let Some(agent) = app.config.agents.get(app.relay_selected_agent) {
                    if !agent.providers.is_empty() {
                        app.relay_view = RelayView::DetailPane;
                        app.relay_edit_field = 0;
                        app.dirty = true;
                    }
                }
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
            KeyCode::Esc => app.leave_settings_detail(),
            KeyCode::Left | KeyCode::Char('h') => {
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
                        app.dirty = true;
                    }
                }
            }
            KeyCode::Char(' ') => {
                app.trigger_provider_test(app.relay_selected_agent, app.relay_selected_provider);
                app.dirty = true;
            }
            _ => {}
        },
    }

    true
}

fn handle_telegram_detail_mode(app: &mut App, key: KeyCode) -> bool {
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
        return true;
    }

    match key {
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
            app.leave_settings_detail();
        }
        KeyCode::Char('r') => restart_telegram_daemon(app),
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
                3 => restart_telegram_daemon(app),
                _ => {}
            }
            app.dirty = true;
        }
        _ => {}
    }
    true
}

fn restart_telegram_daemon(app: &mut App) {
    if let Err(err) = telegram::restart_daemon(&app.config) {
        log_debug!("telegram: restart failed from settings: {}", err);
    }
    app.dirty = true;
}

#[cfg(test)]
mod tests {
    use super::handle_settings_mode;
    use crate::app::state::{Mode, SettingsFocus};
    use crate::app::App;
    use crossterm::event::KeyCode;

    #[test]
    fn settings_search_enter_opens_first_match_directly() {
        let mut app = App::new();
        app.mode = Mode::Settings;
        app.settings_open = true;
        app.settings_focus = SettingsFocus::List;
        app.settings_searching = true;
        app.settings_search = "relay".into();
        app.settings_selected = 0;

        handle_settings_mode(&mut app, KeyCode::Enter);

        assert!(!app.settings_searching);
        assert!(matches!(app.settings_focus, SettingsFocus::Detail));
        assert!(matches!(
            app.current_settings_detail_kind(),
            Some(crate::app::state::SettingsDetailKind::Relay)
        ));
    }

    #[test]
    fn settings_search_enter_with_no_match_stays_in_list() {
        let mut app = App::new();
        app.mode = Mode::Settings;
        app.settings_open = true;
        app.settings_focus = SettingsFocus::List;
        app.settings_searching = true;
        app.settings_search = "no-such-setting".into();
        app.settings_selected = 0;

        handle_settings_mode(&mut app, KeyCode::Enter);

        assert!(!app.settings_searching);
        assert!(matches!(app.settings_focus, SettingsFocus::List));
        assert!(app.current_settings_detail_kind().is_none());
    }
}
