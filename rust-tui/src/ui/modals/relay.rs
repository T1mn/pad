use super::common::{mask_secret_prefix, render_modal_surface, truncate_modal_line};
use crate::app::App;
use crate::i18n::Locale;
use crate::theme::{ProviderConfig, Theme};
use crate::ui::layout::popup_area;
use crate::ui::selection::{
    render::{recommended_list_modal_height, render_selection_surface},
    SelectionItem, SelectionState,
};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

pub fn draw_relay_settings(f: &mut Frame, app: &App) {
    let theme = &app.theme;
    let content_w = match app.relay_view {
        crate::app::state::RelayView::AgentList => 58,
        crate::app::state::RelayView::ProviderList => 76,
        crate::app::state::RelayView::DetailPane => {
            let selected_agent = app.config.agents.get(app.relay_selected_agent);
            if selected_agent
                .map(|agent| agent.name.as_str() == "codex")
                .unwrap_or(false)
            {
                82
            } else {
                68
            }
        }
    };
    let content_h = if app.relay_view == crate::app::state::RelayView::DetailPane {
        let selected_agent = app.config.agents.get(app.relay_selected_agent);
        let prov = selected_agent.and_then(|a| a.providers.get(app.relay_selected_provider));
        let is_codex = selected_agent
            .map(|agent| agent.name.as_str() == "codex")
            .unwrap_or(false);
        let base_lines = if is_codex { 18u16 } else { 14u16 };
        let test_lines = if app.provider_test_in_progress {
            2
        } else if prov.map(|p| p.test_result.is_some()).unwrap_or(false) {
            4
        } else {
            0
        };
        base_lines + test_lines
    } else {
        let count = if app.relay_view == crate::app::state::RelayView::AgentList {
            app.config.agents.len() as u16
        } else {
            app.config
                .agents
                .get(app.relay_selected_agent)
                .map(|a| a.providers.len() as u16)
                .unwrap_or(1)
        };
        recommended_list_modal_height(count, 2, 1, 1).max(12)
    };
    let area = popup_area(content_w, content_h, f.area());
    render_modal_surface(f, area, theme);
    draw_relay_in_area(f, app, area);
}

pub(super) fn draw_relay_in_area(f: &mut Frame, app: &App, area: Rect) {
    if app.relay_view == crate::app::state::RelayView::DetailPane {
        draw_relay_detail_content(f, app, area);
    } else {
        draw_relay_settings_content(f, app, area);
    }
}

fn draw_relay_settings_content(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let l = app.locale;
    let selected_agent = app.config.agents.get(app.relay_selected_agent);
    let (title, items, selected, footer_text) = match app.relay_view {
        crate::app::state::RelayView::AgentList => {
            let items: Vec<SelectionItem> = app
                .config
                .agents
                .iter()
                .map(|agent| {
                    let active_label = agent
                        .active()
                        .map(|p| p.label.clone())
                        .unwrap_or_else(|| "none".to_string());
                    SelectionItem {
                        title: agent.name.clone(),
                        subtitle: Some(format!(
                            "active: {}  ·  {} providers",
                            active_label,
                            agent.providers.len()
                        )),
                        keyword: Some(format!("{} {}", agent.name, active_label)),
                        detail: None,
                        disabled: false,
                    }
                })
                .collect();
            (
                crate::i18n::t(l, "relay.title").to_string(),
                items,
                app.relay_selected_agent,
                crate::i18n::t(l, "relay.footer_agent"),
            )
        }
        crate::app::state::RelayView::ProviderList | crate::app::state::RelayView::DetailPane => {
            let items: Vec<SelectionItem> = if let Some(agent) = selected_agent {
                if agent.providers.is_empty() {
                    vec![SelectionItem {
                        title: crate::i18n::t(l, "relay.empty").to_string(),
                        subtitle: None,
                        keyword: None,
                        detail: None,
                        disabled: true,
                    }]
                } else {
                    agent
                        .providers
                        .iter()
                        .enumerate()
                        .map(|(i, prov)| {
                            let is_active = agent.active_provider == Some(i);
                            SelectionItem {
                                title: provider_list_title(agent.name.as_str(), prov, is_active),
                                subtitle: Some(provider_list_subtitle(agent.name.as_str(), prov)),
                                keyword: Some(format!(
                                    "{} {} {}",
                                    prov.label,
                                    prov.base_url,
                                    prov.test_result.clone().unwrap_or_default()
                                )),
                                detail: None,
                                disabled: false,
                            }
                        })
                        .collect()
                }
            } else {
                vec![]
            };
            (
                format!(
                    "{} / {}",
                    crate::i18n::t(l, "relay.providers_label"),
                    selected_agent.map(|a| a.name.as_str()).unwrap_or("?")
                ),
                items,
                app.relay_selected_provider,
                crate::i18n::t(l, "relay.footer_provider"),
            )
        }
    };

    let mut state = SelectionState {
        selected,
        ..Default::default()
    };
    state.clamp_selected(items.len());
    render_selection_surface(f, area, theme, &title, &items, &state, Some(footer_text));
}

pub fn draw_relay_detail(f: &mut Frame, app: &App) {
    let theme = &app.theme;
    let selected_agent = app.config.agents.get(app.relay_selected_agent);
    let is_codex = selected_agent
        .map(|agent| agent.name.as_str() == "codex")
        .unwrap_or(false);
    let prov = selected_agent.and_then(|a| a.providers.get(app.relay_selected_provider));
    let content_w = if is_codex { 82 } else { 68 };
    let base_lines = if is_codex { 18u16 } else { 14u16 };
    let test_lines = if app.provider_test_in_progress {
        2
    } else if prov.map(|p| p.test_result.is_some()).unwrap_or(false) {
        4
    } else {
        0
    };
    let content_h = base_lines + test_lines;
    let area = popup_area(content_w, content_h, f.area());
    render_modal_surface(f, area, theme);
    draw_relay_detail_content(f, app, area);
}

fn draw_relay_detail_content(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let l = app.locale;

    let selected_agent = app.config.agents.get(app.relay_selected_agent);
    let prov = selected_agent.and_then(|a| a.providers.get(app.relay_selected_provider));
    let prov_label = prov.map(|p| p.label.as_str()).unwrap_or("?");
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
            format!("{} / {}", crate::i18n::t(l, "relay.details"), prov_label),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ))),
        header_area,
    );

    if let (Some(agent), Some(prov)) = (selected_agent, prov) {
        let field = app.relay_edit_field;
        let editing = app.relay_editing;
        let make_val = |idx: usize, val: &str| -> String {
            if editing && field == idx {
                format!("{}|", app.relay_edit_buffer)
            } else if val.is_empty() {
                "-".to_string()
            } else {
                val.to_string()
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
            mask_secret_prefix(prov.codex_auth_token().as_deref().unwrap_or_default(), 10)
        } else if prov.api_key.is_empty() {
            "-".to_string()
        } else if prov.api_key.len() > 12 {
            format!("{}...", &prov.api_key[..12])
        } else {
            prov.api_key.clone()
        };

        let mut detail_lines = vec![
            Line::from(Span::styled(
                crate::i18n::t(l, "relay.label"),
                Style::default()
                    .fg(theme.comment)
                    .add_modifier(Modifier::DIM),
            )),
            Line::from(Span::styled(make_val(0, &prov.label), field_style(0))),
            Line::from(""),
            Line::from(Span::styled(
                crate::i18n::t(l, "relay.base_url"),
                Style::default()
                    .fg(theme.comment)
                    .add_modifier(Modifier::DIM),
            )),
            Line::from(Span::styled(make_val(1, &prov.base_url), field_style(1))),
            Line::from(""),
            Line::from(Span::styled(
                crate::i18n::t(l, "relay.api_key"),
                Style::default()
                    .fg(theme.comment)
                    .add_modifier(Modifier::DIM),
            )),
            Line::from(Span::styled(masked_api_key, field_style(2))),
        ];
        if agent.name == "codex" {
            detail_lines.push(Line::from(""));
            detail_lines.push(Line::from(Span::styled(
                crate::i18n::t(l, "relay.wire_api"),
                Style::default()
                    .fg(theme.comment)
                    .add_modifier(Modifier::DIM),
            )));
            detail_lines.push(Line::from(Span::styled(
                prov.codex_wire_api().to_string(),
                Style::default().fg(theme.fg),
            )));
            detail_lines.push(Line::from(""));
            detail_lines.push(Line::from(Span::styled(
                format!(
                    "auth.json: {}  ·  config.toml: {}",
                    yes_no(prov.codex_auth_token().is_some()),
                    yes_no(!prov.base_url.trim().is_empty())
                ),
                Style::default().fg(theme.comment),
            )));
        }
        append_provider_test_lines(
            &mut detail_lines,
            app.provider_test_in_progress,
            prov.test_status,
            prov.test_http_status,
            prov.test_latency_ms,
            prov.test_result.as_deref(),
            theme,
            l,
        );
        let para = Paragraph::new(detail_lines).wrap(Wrap { trim: false });
        f.render_widget(para, body_area);
    } else {
        let para = Paragraph::new(vec![Line::from(Span::styled(
            crate::i18n::t(l, "relay.no_provider"),
            Style::default().fg(theme.comment),
        ))])
        .wrap(Wrap { trim: false });
        f.render_widget(para, body_area);
    }

    let footer_text = if app.relay_editing {
        crate::i18n::t(l, "relay.footer_edit")
    } else {
        crate::i18n::t(l, "relay.footer_detail")
    };
    let footer = Paragraph::new(Line::from(Span::styled(
        footer_text.to_string(),
        Style::default()
            .fg(theme.comment)
            .add_modifier(Modifier::DIM),
    )));
    f.render_widget(footer, footer_area);
}

fn provider_list_title(agent_name: &str, prov: &ProviderConfig, is_active: bool) -> String {
    let marker = if is_active { "✓ " } else { "" };
    let _ = agent_name;
    format!("{}{}", marker, prov.label)
}

fn provider_list_subtitle(agent_name: &str, prov: &ProviderConfig) -> String {
    let mut parts = Vec::new();
    if !prov.base_url.trim().is_empty() {
        parts.push(truncate_modal_line(&prov.base_url, 28));
    }
    if agent_name == "codex" {
        parts.push(format!(
            "auth {}",
            yes_no(prov.codex_auth_token().is_some())
        ));
    }
    if let Some(ok) = prov.test_status {
        parts.push(if ok {
            "probe ok".to_string()
        } else {
            "probe failed".to_string()
        });
    }
    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join("  |  ")
    }
}

fn yes_no(ready: bool) -> &'static str {
    if ready {
        "ready"
    } else {
        "missing"
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

fn http_status_color(status: u16, theme: &Theme) -> Color {
    match status {
        100..=399 => theme.success,
        400..=499 => theme.warning,
        500..=599 => theme.error,
        _ => theme.comment,
    }
}

fn latency_color(latency_ms: u64, theme: &Theme) -> Color {
    match latency_ms {
        0..=800 => theme.success,
        801..=2500 => theme.warning,
        _ => theme.error,
    }
}
