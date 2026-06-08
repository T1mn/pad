use crate::app::state::RelayPopupMode;
use crate::app::App;
use crate::ui::selection::{render::render_selection_surface, SelectionItem, SelectionState};
use ratatui::{layout::Rect, Frame};

pub(super) fn draw_model_picker_popup(f: &mut Frame, app: &App, popup_rect: Rect) {
    let theme = &app.theme;
    let locale = app.locale;
    let title = if app.relay_popup_mode == RelayPopupMode::OpenCodeDefaultModel {
        crate::i18n::t(locale, "relay.default_model")
    } else {
        crate::i18n::t(locale, "relay.small_model")
    };
    let items = picker_items(app);
    let mut state = SelectionState {
        selected: app.relay_popup_selected,
        ..Default::default()
    };
    state.clamp_selected(items.len());
    render_selection_surface(
        f,
        popup_rect,
        theme,
        title,
        &items,
        &state,
        Some(crate::i18n::t(locale, "relay.footer_model_picker")),
    );
}

fn picker_items(app: &App) -> Vec<SelectionItem> {
    app.config
        .agents
        .get(app.relay_selected_agent)
        .map(|agent| {
            let mut options = agent.opencode_model_options();
            if app.relay_popup_mode == RelayPopupMode::OpenCodeSmallModel {
                options.insert(0, (String::new(), "(none)".to_string()));
            }
            options
                .into_iter()
                .map(|(value, label)| SelectionItem {
                    title: label,
                    value: None,
                    subtitle: Some(value),
                    keyword: None,
                    detail: None,
                    disabled: false,
                })
                .collect()
        })
        .unwrap_or_default()
}
