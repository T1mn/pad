use super::edit::draw_model_edit_popup;
use crate::app::App;
use crate::ui::selection::{render::render_selection_surface, SelectionItem, SelectionState};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub(super) fn draw_models_popup(f: &mut Frame, app: &App, popup_rect: Rect, inner: Rect) {
    let theme = &app.theme;
    let locale = app.locale;
    let title = crate::i18n::t(locale, "relay.models");
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(inner);

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            title,
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ))),
        header,
    );

    let items = model_items(app);
    let mut state = SelectionState {
        selected: app.relay_popup_selected,
        ..Default::default()
    };
    state.clamp_selected(items.len());
    render_selection_surface(f, body, theme, title, &items, &state, None);

    let footer_text = if app.relay_popup_editing {
        crate::i18n::t(locale, "relay.footer_edit")
    } else {
        crate::i18n::t(locale, "relay.footer_models")
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            footer_text,
            Style::default()
                .fg(theme.comment)
                .add_modifier(Modifier::DIM),
        ))),
        footer,
    );

    if app.relay_popup_editing {
        draw_model_edit_popup(f, app, popup_rect);
    }
}

fn model_items(app: &App) -> Vec<SelectionItem> {
    app.config
        .agents
        .get(app.relay_selected_agent)
        .and_then(|agent| agent.providers.get(app.relay_selected_provider))
        .map(|provider| {
            provider
                .models
                .iter()
                .map(|model| SelectionItem {
                    title: model.id.clone(),
                    value: None,
                    subtitle: Some(if model.name.trim().is_empty() {
                        "-".to_string()
                    } else {
                        model.name.clone()
                    }),
                    keyword: Some(format!("{} {}", model.id, model.name)),
                    detail: None,
                    disabled: false,
                })
                .collect()
        })
        .unwrap_or_default()
}
