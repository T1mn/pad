use super::super::edit::persist_relay_config;
use crate::app::state::RelayPopupMode;
use crate::app::App;
use crossterm::event::KeyCode;

pub(super) fn selected_model_picker_index(app: &App, include_none: bool) -> usize {
    let current = app
        .config
        .agents
        .get(app.relay_selected_agent)
        .map(|agent| {
            if include_none {
                agent.small_model.as_str()
            } else {
                agent.default_model.as_str()
            }
        })
        .unwrap_or_default();

    model_picker_options(app)
        .iter()
        .position(|(value, _)| value == current)
        .unwrap_or(0)
}

pub(super) fn handle_opencode_model_picker_popup(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
            app.clear_relay_popup_state();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let max = model_picker_options(app).len().saturating_sub(1);
            if app.relay_popup_selected < max {
                app.relay_popup_selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up if app.relay_popup_selected > 0 => {
            app.relay_popup_selected -= 1;
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            apply_selected_model(app);
        }
        _ => {}
    }
    app.dirty = true;
    true
}

fn apply_selected_model(app: &mut App) {
    let selected = model_picker_options(app)
        .get(app.relay_popup_selected)
        .map(|(value, _)| value.clone())
        .unwrap_or_default();
    let clear_small =
        app.relay_popup_mode == RelayPopupMode::OpenCodeSmallModel && selected.is_empty();
    if let Some(agent) = app.config.agents.get_mut(app.relay_selected_agent) {
        match app.relay_popup_mode {
            RelayPopupMode::OpenCodeDefaultModel if !selected.is_empty() => {
                agent.default_model = selected;
            }
            RelayPopupMode::OpenCodeSmallModel => {
                if clear_small {
                    agent.small_model.clear();
                } else {
                    agent.small_model = selected;
                }
            }
            _ => {}
        }
        agent.repair_opencode_model_refs();
    }
    persist_relay_config(app, app.relay_selected_agent);
    app.clear_relay_popup_state();
}

fn model_picker_options(app: &App) -> Vec<(String, String)> {
    let mut options = app
        .config
        .agents
        .get(app.relay_selected_agent)
        .map(|agent| agent.opencode_model_options())
        .unwrap_or_default();

    if app.relay_popup_mode == RelayPopupMode::OpenCodeSmallModel {
        options.insert(0, (String::new(), "(none)".to_string()));
    }

    options
}
