use crate::app::App;
use crate::relay;
use crossterm::event::KeyCode;

pub(in crate::event::modes::settings) fn handle_trash_detail_mode(
    app: &mut App,
    key: KeyCode,
) -> bool {
    match key {
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => app.leave_settings_detail(),
        KeyCode::Enter | KeyCode::Char(' ') => app.open_trash_threads_view(),
        _ => {}
    }
    true
}

pub(in crate::event::modes::settings) fn handle_auto_refresh_detail_mode(
    app: &mut App,
    key: KeyCode,
) -> bool {
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

pub(in crate::event::modes::settings) fn handle_claude_full_access_detail_mode(
    app: &mut App,
    key: KeyCode,
) -> bool {
    match key {
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => app.leave_settings_detail(),
        KeyCode::Enter | KeyCode::Char(' ') => {
            app.config.agent_permissions.claude_auto_full_access =
                !app.config.agent_permissions.claude_auto_full_access;
            app.config.save();
            relay::apply_runtime_configs(
                &app.config.agents,
                &app.config.agent_permissions,
                &app.config.codex,
            );
            app.dirty = true;
        }
        _ => {}
    }
    true
}

pub(in crate::event::modes::settings) fn handle_preview_mode_detail_mode(
    app: &mut App,
    key: KeyCode,
) -> bool {
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

pub(in crate::event::modes::settings) fn handle_display_mode_detail_mode(
    app: &mut App,
    key: KeyCode,
) -> bool {
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
