mod basic {
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
                app.save_config();
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
                app.save_config();
                relay::apply_runtime_overlays(
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

    pub(in crate::event::modes::settings) fn handle_profile_permission_mode_detail_mode(
        app: &mut App,
        key: KeyCode,
    ) -> bool {
        match key {
            KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => app.leave_settings_detail(),
            KeyCode::Enter | KeyCode::Char(' ') => {
                let next = match app.config.profile.default_permission_mode.as_str() {
                    crate::theme::ProfileConfig::GUARDED => {
                        crate::theme::ProfileConfig::WORKSPACE_FULL_ACCESS
                    }
                    crate::theme::ProfileConfig::WORKSPACE_FULL_ACCESS => {
                        crate::theme::ProfileConfig::SYSTEM_FULL_ACCESS
                    }
                    _ => crate::theme::ProfileConfig::GUARDED,
                };
                app.set_profile_permission_mode(next);
            }
            _ => {}
        }
        true
    }

    pub(in crate::event::modes::settings) fn handle_profile_full_access_detail_mode(
        app: &mut App,
        key: KeyCode,
    ) -> bool {
        match key {
            KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => app.leave_settings_detail(),
            KeyCode::Enter | KeyCode::Char(' ') => app.toggle_profile_full_access(),
            _ => {}
        }
        true
    }

    pub(in crate::event::modes::settings) fn handle_profile_unattended_detail_mode(
        app: &mut App,
        key: KeyCode,
    ) -> bool {
        match key {
            KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => app.leave_settings_detail(),
            KeyCode::Enter | KeyCode::Char(' ') => app.toggle_profile_unattended(),
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
                app.config.preview.mode = "session".to_string();
                app.save_config();
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
}
mod sound;

pub(super) use basic::{
    handle_auto_refresh_detail_mode, handle_claude_full_access_detail_mode,
    handle_display_mode_detail_mode, handle_preview_mode_detail_mode,
    handle_profile_full_access_detail_mode, handle_profile_permission_mode_detail_mode,
    handle_profile_unattended_detail_mode, handle_trash_detail_mode,
};
pub(super) use sound::handle_sound_detail_mode;
