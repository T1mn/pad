mod normal;
mod settings;

use super::text::mode_badge;
use crate::app::state::Mode;
use crate::app::App;
use crate::i18n::t;
use ratatui::text::Span;

pub(super) use normal::compose_status_body;

pub(super) fn mode_span(app: &App) -> Span<'static> {
    let theme = &app.theme;
    let l = app.locale;
    match app.mode {
        Mode::Search => mode_badge(t(l, "mode.search"), theme.mode_search_bg),
        Mode::Settings => mode_badge(t(l, "mode.settings"), theme.accent),
        Mode::TelegramSettings => mode_badge(t(l, "mode.settings"), theme.accent),
        Mode::Help => mode_badge(t(l, "mode.help"), theme.accent),
        Mode::NotificationInbox => mode_badge("INBOX", theme.accent),
        Mode::FilePreview => mode_badge(t(l, "mode.preview"), theme.mode_tree_bg),
        _ if app.sidebar.show_tree => mode_badge(t(l, "mode.tree"), theme.mode_tree_bg),
        _ => mode_badge(t(l, "mode.normal"), theme.mode_normal_bg),
    }
}

pub(super) fn status_body(app: &App, body_width: u16) -> String {
    let l = app.locale;
    match app.mode {
        Mode::Search => settings::search_status_body(app),
        Mode::Settings => {
            if app.settings_searching {
                settings::settings_search_status_body(app)
            } else {
                settings::settings_status_body(app)
            }
        }
        Mode::TelegramSettings => String::from(t(l, "status.settings_nav")),
        Mode::Help => String::from(t(l, "status.help_close")),
        Mode::NotificationInbox => {
            "j/k move | Enter/m read | a all read | d delete | Esc close".to_string()
        }
        Mode::FilePreview => String::from(t(l, "status.preview_nav")),
        _ => compose_status_body(app, body_width),
    }
}
