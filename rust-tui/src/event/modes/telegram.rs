use crate::app::App;
use crate::log_debug;
use crate::telegram;
use crossterm::event::KeyCode;

pub(crate) fn handle_telegram_settings_mode(app: &mut App, key: KeyCode) {
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
            app.mode = crate::app::state::Mode::Settings;
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
