use crate::app::App;
use crossterm::event::KeyCode;

pub(crate) fn handle_agent_style_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.mode = crate::app::state::Mode::Settings;
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
}
