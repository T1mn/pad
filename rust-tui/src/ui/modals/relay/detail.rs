use super::super::common::{mask_secret_prefix, truncate_modal_line};
use super::layout::{http_status_color, latency_color, relay_detail_footer_text, yes_no};
use super::popup::draw_relay_popup;
use crate::app::App;
use crate::i18n::Locale;
use crate::theme::{ProviderConfig, Theme};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

pub(super) fn draw_relay_detail_content(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let locale = app.locale;

    let selected_agent = app.config.agents.get(app.relay_selected_agent);
    let provider =
        selected_agent.and_then(|agent| agent.providers.get(app.relay_selected_provider));
    let provider_label = provider.map(|prov| prov.label.as_str()).unwrap_or("?");
    let inner = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };
    let [header_area, body_area, footer_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(inner);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!(
                "{} / {}",
                crate::i18n::t(locale, "relay.details"),
                provider_label
            ),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ))),
        header_area,
    );

    if let (Some(agent), Some(provider)) = (selected_agent, provider) {
        let field = app.relay_edit_field;
        let editing = app.relay_editing;
        let make_val = |idx: usize, value: &str| -> String {
            if editing && field == idx {
                format!("{}|", app.relay_edit_buffer)
            } else if value.is_empty() {
                "-".to_string()
            } else {
                value.to_string()
            }
        };
        let field_style = |idx: usize| -> Style {
            if field == idx {
                Style::default()
                    .fg(theme.highlight_fg)
                    .bg(theme.highlight_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            }
        };
        let masked_api_key = if editing && field == 2 {
            format!("{}|", app.relay_edit_buffer)
        } else if agent.name == "codex" {
            mask_secret_prefix(
                provider.codex_auth_token().as_deref().unwrap_or_default(),
                10,
            )
        } else if provider.api_key.is_empty() {
            "-".to_string()
        } else if provider.api_key.len() > 12 {
            format!("{}...", &provider.api_key[..12])
        } else {
            provider.api_key.clone()
        };

        let mut detail_lines = if agent.name == "opencode" {
            let models_summary = opencode_models_summary(provider);
            vec![
                detail_line(theme, locale, "relay.label"),
                Line::from(Span::styled(make_val(0, &provider.label), field_style(0))),
                Line::from(""),
                detail_line(theme, locale, "relay.provider_key"),
                Line::from(Span::styled(
                    make_val(1, provider.opencode_provider_key()),
                    field_style(1),
                )),
                Line::from(""),
                detail_line(theme, locale, "relay.npm_package"),
                Line::from(Span::styled(
                    make_val(2, provider.opencode_npm_package()),
                    field_style(2),
                )),
                Line::from(""),
                detail_line(theme, locale, "relay.base_url"),
                Line::from(Span::styled(
                    make_val(3, &provider.base_url),
                    field_style(3),
                )),
                Line::from(""),
                detail_line(theme, locale, "relay.api_key"),
                Line::from(Span::styled(
                    if editing && field == 4 {
                        format!("{}|", app.relay_edit_buffer)
                    } else {
                        masked_api_key
                    },
                    field_style(4),
                )),
                Line::from(""),
                detail_line(theme, locale, "relay.models"),
                Line::from(Span::styled(models_summary, field_style(5))),
                Line::from(""),
                Line::from(Span::styled(
                    format!(
                        "model: {}  ·  small: {}",
                        dash_if_empty(&agent.default_model),
                        dash_if_empty(&agent.small_model)
                    ),
                    Style::default().fg(theme.comment),
                )),
            ]
        } else {
            vec![
                detail_line(theme, locale, "relay.label"),
                Line::from(Span::styled(make_val(0, &provider.label), field_style(0))),
                Line::from(""),
                detail_line(theme, locale, "relay.base_url"),
                Line::from(Span::styled(
                    make_val(1, &provider.base_url),
                    field_style(1),
                )),
                Line::from(""),
                detail_line(theme, locale, "relay.api_key"),
                Line::from(Span::styled(masked_api_key, field_style(2))),
            ]
        };
        if agent.name == "codex" {
            detail_lines.push(Line::from(""));
            detail_lines.push(Line::from(Span::styled(
                format!(
                    "auth.json: {}  ·  pad.config.toml: {}",
                    yes_no(provider.codex_auth_token().is_some()),
                    yes_no(!provider.base_url.trim().is_empty())
                ),
                Style::default().fg(theme.comment),
            )));
        }
        append_provider_test_lines(
            &mut detail_lines,
            app.provider_test_in_progress,
            provider.test_status,
            provider.test_http_status,
            provider.test_latency_ms,
            provider.test_result.as_deref(),
            theme,
            locale,
        );
        let paragraph = Paragraph::new(detail_lines).wrap(Wrap { trim: false });
        f.render_widget(paragraph, body_area);
    } else {
        let paragraph = Paragraph::new(vec![Line::from(Span::styled(
            crate::i18n::t(locale, "relay.no_provider"),
            Style::default().fg(theme.comment),
        ))])
        .wrap(Wrap { trim: false });
        f.render_widget(paragraph, body_area);
    }

    let footer_text = if app.relay_editing {
        crate::i18n::t(locale, "relay.footer_edit")
    } else {
        relay_detail_footer_text(app, locale)
    };
    let footer = Paragraph::new(Line::from(Span::styled(
        footer_text.to_string(),
        Style::default()
            .fg(theme.comment)
            .add_modifier(Modifier::DIM),
    )));
    f.render_widget(footer, footer_area);

    if app.relay_popup_mode != crate::app::state::RelayPopupMode::None {
        draw_relay_popup(f, app, area);
    }
}

fn detail_line(theme: &Theme, locale: Locale, key: &'static str) -> Line<'static> {
    Line::from(Span::styled(
        crate::i18n::t(locale, key),
        Style::default()
            .fg(theme.comment)
            .add_modifier(Modifier::DIM),
    ))
}

fn dash_if_empty(value: &str) -> String {
    if value.trim().is_empty() {
        "-".to_string()
    } else {
        truncate_modal_line(value, 24)
    }
}

fn opencode_models_summary(provider: &ProviderConfig) -> String {
    if provider.models.is_empty() {
        return "-".to_string();
    }
    let preview = provider
        .models
        .iter()
        .take(2)
        .map(|model| {
            if model.name.trim().is_empty() {
                model.id.clone()
            } else {
                format!("{} ({})", model.name, model.id)
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    if provider.models.len() > 2 {
        format!(
            "{}  ·  +{} more",
            truncate_modal_line(&preview, 40),
            provider.models.len() - 2
        )
    } else {
        truncate_modal_line(&preview, 48)
    }
}

#[allow(clippy::too_many_arguments)]
fn append_provider_test_lines(
    lines: &mut Vec<Line<'static>>,
    in_progress: bool,
    status: Option<bool>,
    http_status: Option<u16>,
    latency_ms: Option<u64>,
    result: Option<&str>,
    theme: &Theme,
    locale: Locale,
) {
    if !in_progress && status.is_none() && result.is_none() {
        return;
    }

    lines.push(Line::from(""));

    if in_progress {
        lines.push(Line::from(Span::styled(
            crate::i18n::t(locale, "relay.testing"),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )));
        return;
    }

    let (label, color) = match status {
        Some(true) => ("Reachable", theme.success),
        Some(false) => ("Test Failed", theme.error),
        None => ("Test", theme.comment),
    };
    let mut summary_spans = vec![Span::styled(
        label,
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )];
    if let Some(code) = http_status {
        summary_spans.push(Span::raw("  "));
        summary_spans.push(Span::styled(
            format!("HTTP {}", code),
            Style::default()
                .fg(http_status_color(code, theme))
                .add_modifier(Modifier::BOLD),
        ));
    }
    if let Some(latency_ms) = latency_ms {
        summary_spans.push(Span::raw("  "));
        summary_spans.push(Span::styled(
            format!("{} ms", latency_ms),
            Style::default()
                .fg(latency_color(latency_ms, theme))
                .add_modifier(Modifier::BOLD),
        ));
    }
    lines.push(Line::from(summary_spans));

    if let Some(result) = result {
        for line in result.lines().take(4) {
            lines.push(Line::from(Span::styled(
                truncate_modal_line(line, 72),
                Style::default().fg(theme.comment),
            )));
        }
    }
}
