use super::super::common::render_modal_surface;
use crate::app::state::RelayPopupMode;
use crate::app::App;
use crate::ui::layout::popup_area;
use crate::ui::selection::{render::render_selection_surface, SelectionItem, SelectionState};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub(super) fn draw_relay_popup(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let locale = app.locale;
    let popup_rect = match app.relay_popup_mode {
        RelayPopupMode::OpenCodeModels => popup_area(60, 16, area),
        RelayPopupMode::OpenCodeDefaultModel | RelayPopupMode::OpenCodeSmallModel => {
            popup_area(66, 14, area)
        }
        RelayPopupMode::None => return,
    };
    render_modal_surface(f, popup_rect, theme);

    let inner = Rect {
        x: popup_rect.x + 2,
        y: popup_rect.y + 1,
        width: popup_rect.width.saturating_sub(4),
        height: popup_rect.height.saturating_sub(2),
    };

    match app.relay_popup_mode {
        RelayPopupMode::OpenCodeModels => {
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

            let items: Vec<SelectionItem> = app
                .config
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
                .unwrap_or_default();
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
                let edit_area = popup_area(44, 5, popup_rect);
                render_modal_surface(f, edit_area, theme);
                let edit_inner = Rect {
                    x: edit_area.x + 2,
                    y: edit_area.y + 1,
                    width: edit_area.width.saturating_sub(4),
                    height: edit_area.height.saturating_sub(2),
                };
                let label = if app.relay_popup_field == 0 {
                    crate::i18n::t(locale, "relay.model_id")
                } else {
                    crate::i18n::t(locale, "relay.model_name")
                };
                let lines = vec![
                    Line::from(Span::styled(
                        label,
                        Style::default()
                            .fg(theme.comment)
                            .add_modifier(Modifier::DIM),
                    )),
                    Line::from(Span::styled(
                        format!("{}|", app.relay_popup_buffer),
                        Style::default()
                            .fg(theme.highlight_fg)
                            .bg(theme.highlight_bg)
                            .add_modifier(Modifier::BOLD),
                    )),
                ];
                f.render_widget(Paragraph::new(lines), edit_inner);
            }
        }
        RelayPopupMode::OpenCodeDefaultModel | RelayPopupMode::OpenCodeSmallModel => {
            let title = if app.relay_popup_mode == RelayPopupMode::OpenCodeDefaultModel {
                crate::i18n::t(locale, "relay.default_model")
            } else {
                crate::i18n::t(locale, "relay.small_model")
            };
            let items: Vec<SelectionItem> = app
                .config
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
                .unwrap_or_default();
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
        RelayPopupMode::None => {}
    }
}
