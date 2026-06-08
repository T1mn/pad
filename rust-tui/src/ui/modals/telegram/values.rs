use crate::app::App;
use crate::runtime_status;

use super::labels::{restart_value, telegram_label};

pub(super) struct TelegramRowValue {
    pub(super) field_idx: usize,
    pub(super) name: String,
    pub(super) value: String,
    pub(super) editable: bool,
}

pub(super) fn telegram_rows(app: &App) -> Vec<TelegramRowValue> {
    let locale = app.locale;
    vec![
        TelegramRowValue {
            field_idx: 0,
            name: telegram_label(locale, "enabled"),
            value: enabled_value(app),
            editable: true,
        },
        TelegramRowValue {
            field_idx: 1,
            name: telegram_label(locale, "bot_token"),
            value: token_value(app),
            editable: true,
        },
        TelegramRowValue {
            field_idx: 2,
            name: telegram_label(locale, "chat_id"),
            value: chat_value(app),
            editable: true,
        },
        TelegramRowValue {
            field_idx: 3,
            name: telegram_label(locale, "restart_bot"),
            value: restart_value(locale),
            editable: true,
        },
        TelegramRowValue {
            field_idx: 99,
            name: telegram_label(locale, "bot_username"),
            value: username_value(app),
            editable: false,
        },
        TelegramRowValue {
            field_idx: 99,
            name: telegram_label(locale, "pad_status"),
            value: runtime_status::describe_status(&crate::paths::pad_status_path()),
            editable: false,
        },
        TelegramRowValue {
            field_idx: 99,
            name: telegram_label(locale, "bot_status"),
            value: runtime_status::describe_status(&crate::paths::telegram_bot_status_path()),
            editable: false,
        },
    ]
}

fn enabled_value(app: &App) -> String {
    if app.config.telegram.enabled {
        crate::i18n::t(app.locale, "settings.on").to_string()
    } else {
        crate::i18n::t(app.locale, "settings.off").to_string()
    }
}

fn token_value(app: &App) -> String {
    if app.telegram_editing && app.telegram_selected_field == 1 {
        format!("{}|", app.telegram_edit_buffer)
    } else {
        mask_secret(&app.config.telegram.bot_token)
    }
}

fn chat_value(app: &App) -> String {
    if app.telegram_editing && app.telegram_selected_field == 2 {
        format!("{}|", app.telegram_edit_buffer)
    } else if app.config.telegram.chat_id.is_empty() {
        "(empty)".to_string()
    } else {
        app.config.telegram.chat_id.clone()
    }
}

fn username_value(app: &App) -> String {
    if app.config.telegram.bot_username.is_empty() {
        "(unknown)".to_string()
    } else {
        format!("@{}", app.config.telegram.bot_username)
    }
}

fn mask_secret(secret: &str) -> String {
    if secret.is_empty() {
        return "(empty)".to_string();
    }
    let chars = secret.chars().collect::<Vec<_>>();
    if chars.len() <= 10 {
        return "*".repeat(chars.len());
    }
    let head = chars.iter().take(4).collect::<String>();
    let tail = chars
        .iter()
        .rev()
        .take(4)
        .copied()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("{}…{}", head, tail)
}
