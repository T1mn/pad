use crate::app::App;
use crate::log_debug;
use crate::relay;
use crossterm::event::KeyCode;

pub(super) fn handle_trash_detail_mode(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => app.leave_settings_detail(),
        KeyCode::Enter | KeyCode::Char(' ') => app.open_trash_threads_view(),
        _ => {}
    }
    true
}

pub(super) fn handle_auto_refresh_detail_mode(app: &mut App, key: KeyCode) -> bool {
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

pub(super) fn handle_codex_settings_detail_mode(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => app.leave_settings_detail(),
        KeyCode::Char('j') | KeyCode::Down => {
            if app.codex_settings_selected < 12 {
                app.codex_settings_selected += 1;
            }
            app.dirty = true;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.codex_settings_selected > 0 {
                app.codex_settings_selected -= 1;
            }
            app.dirty = true;
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            match app.codex_settings_selected {
                0 => {
                    app.config.agent_permissions.codex_auto_full_access =
                        !app.config.agent_permissions.codex_auto_full_access;
                }
                1 => {
                    app.config.codex.fast_mode = !app.config.codex.fast_mode;
                }
                2 => {
                    app.config.codex.goals = !app.config.codex.goals;
                }
                3 => {
                    app.config.codex.multi_agent = !app.config.codex.multi_agent;
                }
                4 => {
                    app.config.codex.web_search = match app.config.codex.web_search.as_str() {
                        "cached" => "live".to_string(),
                        "live" => "disabled".to_string(),
                        "disabled" => "default".to_string(),
                        _ => "cached".to_string(),
                    };
                }
                5 => {
                    app.config.codex.status_line_model_with_reasoning =
                        !app.config.codex.status_line_model_with_reasoning;
                }
                6 => {
                    app.config.codex.status_line_context_remaining =
                        !app.config.codex.status_line_context_remaining;
                }
                7 => {
                    app.config.codex.status_line_current_dir =
                        !app.config.codex.status_line_current_dir;
                }
                8 => {
                    app.config.codex.jailbreak_prompt_file =
                        !app.config.codex.jailbreak_prompt_file;
                }
                9 => {
                    app.config.codex.index_prompt_file = !app.config.codex.index_prompt_file;
                }
                10 => {
                    app.config.codex.title_summary = !app.config.codex.title_summary;
                }
                11 => {
                    app.config.codex.show_qa_preview = !app.config.codex.show_qa_preview;
                }
                12 => {
                    app.trigger_codex_cli_version_check();
                    return true;
                }
                _ => {}
            }
            app.config.save();
            relay::apply_runtime_configs(
                &app.config.agents,
                &app.config.agent_permissions,
                &app.config.codex,
            );
            app.dirty = true;
        }
        KeyCode::Char('u') if app.codex_settings_selected == 12 => {
            app.trigger_codex_cli_update();
            app.dirty = true;
        }
        _ => {}
    }
    true
}

pub(super) fn handle_claude_full_access_detail_mode(app: &mut App, key: KeyCode) -> bool {
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

pub(super) fn handle_sound_detail_mode(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => app.leave_settings_detail(),
        KeyCode::Char('j') | KeyCode::Down => {
            if app.sound_settings_selected < 8 {
                app.sound_settings_selected += 1;
            }
            app.dirty = true;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.sound_settings_selected > 0 {
                app.sound_settings_selected -= 1;
            }
            app.dirty = true;
        }
        KeyCode::Enter => apply_sound_settings_action(app, false),
        KeyCode::Char(' ') => apply_sound_settings_action(app, true),
        _ => {}
    }
    true
}

pub(super) fn handle_preview_mode_detail_mode(app: &mut App, key: KeyCode) -> bool {
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

pub(super) fn handle_display_mode_detail_mode(app: &mut App, key: KeyCode) -> bool {
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

fn apply_sound_settings_action(app: &mut App, preview: bool) {
    match (app.sound_settings_selected, preview) {
        (0, _) => {
            app.config.sound.enabled = !app.config.sound.enabled;
            app.config.save();
        }
        (1, _) => {
            app.config.sound.completion.enabled = !app.config.sound.completion.enabled;
            app.config.save();
        }
        (2, true) => preview_sound_preset(&app.config.sound.completion.preset),
        (2, false) => {
            cycle_sound_preset(&mut app.config.sound.completion.preset);
            app.config.save();
        }
        (3, _) => {
            app.config.sound.approval.enabled = !app.config.sound.approval.enabled;
            app.config.save();
        }
        (4, true) => preview_sound_preset(&app.config.sound.approval.preset),
        (4, false) => {
            cycle_sound_preset(&mut app.config.sound.approval.preset);
            app.config.save();
        }
        (5, _) => {
            app.config.sound.timeout.enabled = !app.config.sound.timeout.enabled;
            app.config.save();
        }
        (6, true) => preview_sound_preset(&app.config.sound.timeout.preset),
        (6, false) => {
            cycle_sound_preset(&mut app.config.sound.timeout.preset);
            app.config.save();
        }
        (7, _) => {
            app.config.sound.failure.enabled = !app.config.sound.failure.enabled;
            app.config.save();
        }
        (8, true) => preview_sound_preset(&app.config.sound.failure.preset),
        (8, false) => {
            cycle_sound_preset(&mut app.config.sound.failure.preset);
            app.config.save();
        }
        _ => {}
    }
    app.dirty = true;
}

fn cycle_sound_preset(current: &mut String) {
    let presets = crate::sound::preset_ids();
    let current_index = presets
        .iter()
        .position(|preset| *preset == current)
        .unwrap_or(0);
    let next_index = (current_index + 1) % presets.len();
    *current = presets[next_index].to_string();
}

fn preview_sound_preset(preset_id: &str) {
    if let Err(err) = crate::sound::preview_preset(preset_id) {
        log_debug!(
            "sound: preset preview failed preset={} err={}",
            preset_id,
            err
        );
    }
}
