use crate::app::App;
use crate::i18n::t;
use crate::ui::selection::{render::render_selection_surface, SelectionItem, SelectionState};
use ratatui::layout::Rect;
use ratatui::Frame;

pub(in crate::ui::modals::settings) fn draw_language_detail(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let locale = app.locale;
    let current_locale = crate::i18n::Locale::from_str(&app.config.language);
    let items: Vec<SelectionItem> = App::available_locales()
        .iter()
        .map(|entry| {
            let is_current = *entry == current_locale;
            SelectionItem {
                title: if is_current {
                    format!("✓ {}", entry.display_name())
                } else {
                    entry.display_name().to_string()
                },
                value: None,
                subtitle: Some(entry.as_str().to_string()),
                keyword: Some(format!("{} {}", entry.display_name(), entry.as_str())),
                detail: None,
                disabled: false,
            }
        })
        .collect();
    let mut state = SelectionState {
        selected: app.language_selected,
        ..Default::default()
    };
    state.clamp_selected(items.len());
    render_selection_surface(
        f,
        area,
        theme,
        t(locale, "settings.language"),
        &items,
        &state,
        Some("j/k move · Enter apply · Esc back"),
    );
}
