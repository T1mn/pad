use crate::app::App;
use crate::i18n::t;
use crate::ui::selection::{render::render_selection_surface, SelectionItem, SelectionState};
use ratatui::layout::Rect;
use ratatui::Frame;

pub(in crate::ui::modals::settings) fn draw_theme_detail(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let locale = app.locale;
    let items: Vec<SelectionItem> = App::available_themes()
        .iter()
        .map(|(name, desc)| {
            let is_current = *name == app.config.theme;
            SelectionItem {
                title: if is_current {
                    format!("✓ {}", name)
                } else {
                    name.to_string()
                },
                value: None,
                subtitle: Some(if is_current {
                    format!("{}  ·  current", desc)
                } else {
                    (*desc).to_string()
                }),
                keyword: Some(format!("{} {}", name, desc)),
                detail: None,
                disabled: false,
            }
        })
        .collect();
    let mut state = SelectionState {
        selected: app.theme_selected,
        ..Default::default()
    };
    state.clamp_selected(items.len());
    render_selection_surface(
        f,
        area,
        theme,
        &format!("{} [{}]", t(locale, "settings.theme"), app.theme.name),
        &items,
        &state,
        Some("j/k move · Enter apply · Esc back"),
    );
}
