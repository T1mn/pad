use crate::app::App;
use crate::log_debug;
use crossterm::event::KeyCode;

const SOUND_SETTINGS_LAST_INDEX: usize = 8;

pub(in crate::event::modes::settings) fn handle_sound_detail_mode(
    app: &mut App,
    key: KeyCode,
) -> bool {
    match key {
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => app.leave_settings_detail(),
        KeyCode::Char('j') | KeyCode::Down => select_next_sound_item(app),
        KeyCode::Char('k') | KeyCode::Up => select_previous_sound_item(app),
        KeyCode::Enter => apply_sound_settings_action(app, false),
        KeyCode::Char(' ') => apply_sound_settings_action(app, true),
        _ => {}
    }
    true
}

fn select_next_sound_item(app: &mut App) {
    if app.sound_settings_selected < SOUND_SETTINGS_LAST_INDEX {
        app.sound_settings_selected += 1;
    }
    app.dirty = true;
}

fn select_previous_sound_item(app: &mut App) {
    if app.sound_settings_selected > 0 {
        app.sound_settings_selected -= 1;
    }
    app.dirty = true;
}

fn apply_sound_settings_action(app: &mut App, preview: bool) {
    let changed = match (app.sound_settings_selected, preview) {
        (0, _) => {
            toggle_sound_enabled(app);
            true
        }
        (1, _) => {
            toggle_completion_sound(app);
            true
        }
        (2, true) => {
            preview_sound_preset(&app.config.sound.completion.preset);
            false
        }
        (2, false) => cycle_sound_preset(&mut app.config.sound.completion.preset),
        (3, _) => {
            toggle_approval_sound(app);
            true
        }
        (4, true) => {
            preview_sound_preset(&app.config.sound.approval.preset);
            false
        }
        (4, false) => cycle_sound_preset(&mut app.config.sound.approval.preset),
        (5, _) => {
            toggle_timeout_sound(app);
            true
        }
        (6, true) => {
            preview_sound_preset(&app.config.sound.timeout.preset);
            false
        }
        (6, false) => cycle_sound_preset(&mut app.config.sound.timeout.preset),
        (7, _) => {
            toggle_failure_sound(app);
            true
        }
        (8, true) => {
            preview_sound_preset(&app.config.sound.failure.preset);
            false
        }
        (8, false) => cycle_sound_preset(&mut app.config.sound.failure.preset),
        _ => false,
    };
    if changed {
        app.config.save();
    }
    app.dirty = true;
}

fn toggle_sound_enabled(app: &mut App) {
    app.config.sound.enabled = !app.config.sound.enabled;
}

fn toggle_completion_sound(app: &mut App) {
    app.config.sound.completion.enabled = !app.config.sound.completion.enabled;
}

fn toggle_approval_sound(app: &mut App) {
    app.config.sound.approval.enabled = !app.config.sound.approval.enabled;
}

fn toggle_timeout_sound(app: &mut App) {
    app.config.sound.timeout.enabled = !app.config.sound.timeout.enabled;
}

fn toggle_failure_sound(app: &mut App) {
    app.config.sound.failure.enabled = !app.config.sound.failure.enabled;
}

fn cycle_sound_preset(current: &mut String) -> bool {
    let presets = crate::sound::preset_ids();
    let current_index = presets
        .iter()
        .position(|preset| *preset == current)
        .unwrap_or(0);
    let next_index = (current_index + 1) % presets.len();
    *current = presets[next_index].to_string();
    true
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
